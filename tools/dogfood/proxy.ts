// Сценарий proxy-репозитория crates.io: настоящий cargo собирает проект с зависимостью с crates.io
// через `[source.crates-io] replace-with`, второй прогон с пустым CARGO_HOME идёт из кэша crane.
import { accessLines } from "./audit.ts";
import { type CargoClient, ROOT_CERT } from "./cargo.ts";
import { tail } from "./exec.ts";
import type { Stand } from "./stand.ts";

const REGISTRY = "crane-proxy";
const REGISTRY_ENV = REGISTRY.toUpperCase().replace("-", "_");
const HTTP_OK = 200;
const HTTP_NOT_FOUND = 404;
const MISSING_VERSION = "99.0.0";

const CRATE = "form_urlencoded";
const VERSION = "1.2.1";
const PRINTED = "a+b";

const manifest = (name: string): string =>
  [
    "[package]",
    `name = "${name}"`,
    'version = "0.1.0"',
    'edition = "2021"',
    "",
    "[dependencies]",
    `${CRATE} = "=${VERSION}"`,
    "",
  ].join("\n");

const sourceConfig = (url: string): string =>
  [
    "[source.crates-io]",
    `replace-with = "${REGISTRY}"`,
    "",
    `[registries.${REGISTRY}]`,
    `index = "sparse+${url}/index/"`,
    "",
  ].join("\n");

/** Запросы, у которых проверяется `cache`: индекс и архив крейта и его транзитивной зависимости. */
const watched = (base: string): RegExp[] => [
  new RegExp(`^${base}/index/fo/rm/form_urlencoded$`),
  new RegExp(`^${base}/api/v1/crates/${CRATE}/1\\.2\\.1/download$`),
  new RegExp(`^${base}/index/pe/rc/percent-encoding$`),
  new RegExp(`^${base}/api/v1/crates/percent-encoding/[^/]+/download$`),
];

/**
 * Собирает проект дважды с пустыми CARGO_HOME и сверяет по журналу crane, откуда пришёл каждый ответ:
 * первый прогон `miss`, второй `hit`. Заодно: версия вне индекса даёт 404.
 * `repo` у каждой цели cargo свой: кэш привязан к репозиторию, поэтому первый прогон не зависит от прежних.
 */
export function proxyScenario(
  stand: Stand,
  client: CargoClient,
  minor: number,
  repo: string,
): void {
  const base = `/cargo/${repo}`;
  const url = `https://localhost${base}`;
  const project = `proxyapp_${stand.id}_${minor}`;
  client.write(`${project}/Cargo.toml`, manifest(project));
  client.write(`${project}/.cargo/config.toml`, sourceConfig(url));
  client.write(
    `${project}/src/main.rs`,
    `fn main() {\n    println!("{}", ${CRATE}::byte_serialize(b"a b").collect::<String>());\n}\n`,
  );

  const build = (run: number): void => {
    const home = `/tmp/proxy-${minor}-${run}`;
    const result = client.exec(["cargo", "run", "-q"], {
      cwd: `/work/${project}`,
      env: {
        CARGO_HOME: home,
        CARGO_TARGET_DIR: `${home}-target`,
        [`CARGO_REGISTRIES_${REGISTRY_ENV}_TOKEN`]: stand.token,
        [`CARGO_REGISTRIES_${REGISTRY_ENV}_CREDENTIAL_PROVIDER`]: "cargo:token",
      },
    });
    if (result.status !== 0 || result.stdout.trim() !== PRINTED) {
      throw new Error(
        `сборка ${run}: код ${result.status}, вывод «${result.stdout.trim()}»\n${tail(result.stderr)}`,
      );
    }
  };

  const seen = (): number => accessLines(stand.craneLogs()).length;
  const first = seen();
  build(1);
  const second = seen();
  build(2);
  const lines = accessLines(stand.craneLogs());
  expectCache(watched(base), lines.slice(first, second), "miss");
  expectCache(watched(base), lines.slice(second), "hit");

  const status = client.exec(
    [
      "curl",
      "-sS",
      "-o",
      "/dev/null",
      "-w",
      "%{http_code}",
      "--cacert",
      ROOT_CERT,
      "-K",
      "-",
      `${url}/api/v1/crates/${CRATE}/${MISSING_VERSION}/download`,
    ],
    { input: `header = "Authorization: ${stand.token}"\n` },
  );
  if (status.stdout.trim() !== String(HTTP_NOT_FOUND)) {
    throw new Error(`версия вне индекса: ожидался 404, получен ${status.stdout.trim()}`);
  }
}

/** Для каждого пути есть успешная строка, и все такие строки несут ожидаемое значение `cache`. */
function expectCache(
  paths: RegExp[],
  lines: ReturnType<typeof accessLines>,
  want: "miss" | "hit",
): void {
  for (const path of paths) {
    const caches = lines
      .filter((line) => line.status === HTTP_OK && path.test(line.path))
      .map((line) => line.cache);
    const ok =
      want === "miss"
        ? caches.includes("miss")
        : caches.length > 0 && caches.every((cache) => cache === "hit");
    if (!ok) {
      throw new Error(`${path}: ожидался cache=${want}, в журнале [${caches.join(", ")}]`);
    }
  }
}
