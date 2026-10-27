// Сценарий proxy-репозитория crates.io: настоящий cargo собирает проект с зависимостью с crates.io
// через `[source.crates-io] replace-with`, второй прогон с пустым CARGO_HOME идёт из кэша crane.
import { accessLines } from "./audit.ts";
import { type CargoClient, ROOT_CERT } from "./cargo.ts";
import { tail } from "./exec.ts";
import { PROXY_REPO, type Stand } from "./stand.ts";

const REGISTRY = "crane-proxy";
const REGISTRY_ENV = REGISTRY.toUpperCase().replace("-", "_");
const BASE = `/cargo/${PROXY_REPO}`;
const URL = `https://localhost${BASE}`;
const HTTP_OK = 200;
const HTTP_NOT_FOUND = 404;
const MISSING_VERSION = "99.0.0";

/** Зависимость с crates.io, её вывод и запросы, которые обязаны пройти через proxy. */
interface Case {
  name: string;
  version: string;
  /** Выражение Rust, печатаемое сборкой. */
  call: string;
  printed: string;
  /** Пути индекса и download, у которых проверяется `cache`. */
  paths: RegExp[];
}

const download = (name: string, version: string): RegExp =>
  new RegExp(`^${BASE}/api/v1/crates/${name}/${version}/download$`);

/**
 * Разные крейты на разные версии cargo: кэш crane общий на весь стенд, и второй cargo на тех же
 * крейтах получил бы `hit` вместо `miss` в первом прогоне.
 */
const CASES: Case[] = [
  {
    name: "form_urlencoded",
    version: "1.2.1",
    call: 'form_urlencoded::byte_serialize(b"a b").collect::<String>()',
    printed: "a+b",
    paths: [
      new RegExp(`^${BASE}/index/fo/rm/form_urlencoded$`),
      download("form_urlencoded", "1\\.2\\.1"),
      new RegExp(`^${BASE}/index/pe/rc/percent-encoding$`),
      download("percent-encoding", "[^/]+"),
    ],
  },
  {
    name: "fnv",
    version: "1.0.7",
    call: '{ use std::hash::Hasher; let mut h = fnv::FnvHasher::default(); h.write(b"a"); h.finish() }',
    printed: "12638187200555641996",
    paths: [new RegExp(`^${BASE}/index/3/f/fnv$`), download("fnv", "1\\.0\\.7")],
  },
];

const manifest = (name: string, c: Case): string =>
  [
    "[package]",
    `name = "${name}"`,
    'version = "0.1.0"',
    'edition = "2021"',
    "",
    "[dependencies]",
    `${c.name} = "=${c.version}"`,
    "",
  ].join("\n");

const sourceConfig = [
  "[source.crates-io]",
  `replace-with = "${REGISTRY}"`,
  "",
  `[registries.${REGISTRY}]`,
  `index = "sparse+${URL}/index/"`,
  "",
].join("\n");

/**
 * Собирает проект дважды с пустыми CARGO_HOME и сверяет по журналу crane, откуда пришёл каждый ответ:
 * первый прогон `miss`, второй `hit`. Заодно: версия вне индекса даёт 404.
 */
export function proxyScenario(
  stand: Stand,
  client: CargoClient,
  minor: number,
  index: number,
): void {
  const c = CASES[index];
  if (c === undefined) {
    throw new Error(`для версии cargo №${index} нет крейта в сценарии proxy`);
  }
  const project = `proxyapp_${stand.id}_${minor}`;
  client.write(`${project}/Cargo.toml`, manifest(project, c));
  client.write(`${project}/.cargo/config.toml`, sourceConfig);
  client.write(`${project}/src/main.rs`, `fn main() {\n    println!("{}", ${c.call});\n}\n`);

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
    if (result.status !== 0 || result.stdout.trim() !== c.printed) {
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
  expectCache(c.paths, lines.slice(first, second), "miss");
  expectCache(c.paths, lines.slice(second), "hit");

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
      `${URL}/api/v1/crates/${c.name}/${MISSING_VERSION}/download`,
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
