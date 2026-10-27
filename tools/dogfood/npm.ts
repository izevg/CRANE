// Настоящий npm в контейнере node:current: клиент ходит на https://localhost, как оператор на своей машине.
// Hosted принимает только имена с областью (`@dogfood/..`); форма без области покрыта proxy.
// Сценарий hosted: публикация, dist-теги, deprecate, установка, unpublish по политике. Сценарий proxy:
// установка пакета с npmjs дважды (промах, затем попадание). Токен идёт только через окружение docker CLI.
import { accessLines, type Egress, type Expected, egressLines } from "./audit.ts";
import { authorizedCurl, ROOT_CERT } from "./cargo.ts";
import { type Client, startClient } from "./client.ts";
import { docker, log, MS_PER_SECOND, type Result, run, SECONDS_PER_MINUTE, tail } from "./exec.ts";
import { expectCache } from "./proxy.ts";
import { npmProxyRepo, type Stand } from "./stand.ts";

export interface NpmClient extends Client {
  /** Версия npm после `npm install -g npm@latest`. */
  version: string;
}

const BASE_IMAGE = "node:current-bookworm-slim";
const BUILD_TIMEOUT_MINUTES = 10;
const BUILD_TIMEOUT_MS = BUILD_TIMEOUT_MINUTES * SECONDS_PER_MINUTE * MS_PER_SECOND;
const READY_SECONDS = 60;
const TOKEN_ENV = "NPM_TOKEN";
const HOSTED_REPO = "internal";
const HOST = "https://localhost";
const HTTP_CONFLICT = 409;
const SCOPE = "@dogfood";
const LEADING_SLASHES = /^\/+/;

const CLIENT_ENV = {
  NODE_EXTRA_CA_CERTS: ROOT_CERT,
  NPM_CONFIG_UPDATE_NOTIFIER: "false",
  NPM_CONFIG_FUND: "false",
};

// Образ собирается вне сети стенда (там есть интернет), клиент потом запускается в сети Caddy.
// В node-slim нет curl, а он нужен для проверки готовности с токеном через stdin.
const DOCKERFILE = [
  `FROM ${BASE_IMAGE}`,
  "RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \\",
  " && rm -rf /var/lib/apt/lists/*",
  "RUN npm install -g npm@latest",
  "",
].join("\n");

/** Ссылка на переменную окружения в .npmrc: npm раскрывает её сам, значение в файл не попадает. */
const TOKEN_REF = `\${${TOKEN_ENV}}`;

/** Адрес репозитория с завершающим слэшем: по этому префиксу npm выбирает токен. */
const repoUrl = (repo: string): string => `${HOST}/npm/${repo}/`;

/** .npmrc проекта: реестр по умолчанию, при необходимости реестр области и токены обоих репозиториев. */
export function npmrc(registry: string, scoped?: { scope: string; registry: string }): string {
  const auth = [HOSTED_REPO, npmProxyRepo].map(
    (repo) => `${repoUrl(repo).replace("https:", "")}:_authToken=${TOKEN_REF}`,
  );
  return [
    `registry=${registry}`,
    ...(scoped === undefined ? [] : [`${scoped.scope}:registry=${scoped.registry}`]),
    ...auth,
    "",
  ].join("\n");
}

export function packageJson(name: string, version: string): string {
  return JSON.stringify(
    { name, version, main: "index.js", license: "MIT", description: "CRANE dogfood" },
    null,
    2,
  );
}

/** purl, как его строит сервер: `@` области кодируется, без версии — событие над пакетом целиком. */
export const npmPurl = (name: string, version?: string): string =>
  `pkg:npm/${name.replaceAll("@", "%40")}${version === undefined ? "" : `@${version}`}`;

function parseJson(raw: string): unknown {
  try {
    return JSON.parse(raw);
  } catch {
    return undefined;
  }
}

const sortedEntries = (record: object): [string, unknown][] =>
  Object.entries(record).sort(([a], [b]) => a.localeCompare(b));

/**
 * npm 12 печатает `view <имя> <поле> --json` для одной версии массивом из одного элемента, если значение
 * поля не массив (`[{"latest":..}]`); прежние npm печатали сам объект. Принимаем обе формы.
 */
export function unwrapSingle(value: unknown): unknown {
  return Array.isArray(value) && value.length === 1 ? value[0] : value;
}

/** `npm view .. dist-tags --json` совпадает с ожидаемым словарём тегов точно. */
export function sameTags(raw: string, want: Record<string, string>): boolean {
  const got = unwrapSingle(parseJson(raw));
  if (typeof got !== "object" || got === null || Array.isArray(got)) {
    return false;
  }
  return JSON.stringify(sortedEntries(got)) === JSON.stringify(sortedEntries(want));
}

/** `npm view .. versions --json`: у единственной версии npm печатает строку, а не массив. */
export function versionsOf(raw: string): string[] {
  const got = unwrapSingle(parseJson(raw));
  if (typeof got === "string") {
    return [got];
  }
  return Array.isArray(got) ? got.filter((item): item is string => typeof item === "string") : [];
}

export const hasDeprecationWarning = (stderr: string, spec: string, message: string): boolean =>
  stderr.includes(`deprecated ${spec}: ${message}`);

/** Выборки из upstream по пути; ведущие слэши не важны (в журнале путь без них). */
export function upstreamFetches(egress: Egress[], path: string): number {
  const bare = (value: string): string => value.replace(LEADING_SLASHES, "");
  return egress.filter((line) => bare(line.path) === bare(path)).length;
}

/**
 * Собирает образ с последним npm, запускает клиент и ждёт, пока Caddy выпустит сертификат,
 * а crane ответит на ping. Токен идёт через stdin, не через argv.
 */
export function startNpm(stand: Stand): NpmClient {
  const image = `crane-dogfood-npm:${stand.id}`;
  log(`клиент ${BASE_IMAGE}: сборка образа с npm@latest`);
  docker(["build", "--pull", "-q", "-t", image, "-"], {
    input: DOCKERFILE,
    timeoutMs: BUILD_TIMEOUT_MS,
  });
  const removeImage = (): void => {
    run("docker", ["image", "rm", image]);
  };
  let client: Client;
  try {
    client = startClient(stand, image, CLIENT_ENV);
  } catch (error) {
    removeImage();
    throw error;
  }
  try {
    const ready = authorizedCurl(
      client,
      stand.token,
      [
        "-fsS",
        "--retry",
        String(READY_SECONDS),
        "--retry-delay",
        "1",
        "--retry-all-errors",
        "--retry-max-time",
        String(READY_SECONDS),
        `${repoUrl(HOSTED_REPO)}-/ping`,
      ],
      "Bearer",
    );
    if (ready.status !== 0) {
      throw new Error(`npm-реестр не ответил за ${READY_SECONDS} с: ${tail(ready.stderr)}`);
    }
    const version = client.exec(["npm", "--version"]).stdout.trim();
    return {
      ...client,
      version,
      stop() {
        client.stop();
        removeImage();
      },
    };
  } catch (error) {
    client.stop();
    removeImage();
    throw error;
  }
}

const MESSAGE = "dogfood: use 2.x";
const CONSTANT = "crane-npm-dogfood";
const FIRST = "1.0.0";
const BETA = "1.1.0-beta.0";
const PROXY_PACKAGE = "is-number";
const PROXY_VERSION = "7.0.0";
const PROXY_PACKUMENT = new RegExp(`^/npm/${npmProxyRepo}/${PROXY_PACKAGE}$`);
const PROXY_TARBALL = new RegExp(
  `^/npm/${npmProxyRepo}/${PROXY_PACKAGE}/-/${PROXY_PACKAGE}-7\\.0\\.0\\.tgz$`,
);
const PROXY_TARBALL_PATH = `${PROXY_PACKAGE}/-/${PROXY_PACKAGE}-${PROXY_VERSION}.tgz`;

/** Всё, что нужно шагам сценария: запуск npm с токеном в окружении и проверка кода выхода. */
interface Tools {
  client: NpmClient;
  stand: Stand;
  agent: string;
  /** Выполняет `npm <args>` в /work/<dir>; код выхода не проверяет. */
  npm(dir: string, args: string[]): Result;
  /** То же, но ненулевой код — ошибка с хвостом stderr. */
  must(dir: string, args: string[]): Result;
  /** Новый пустой каталог кэша npm. */
  cache(): string;
}

function tools(stand: Stand, client: NpmClient): Tools {
  let caches = 0;
  const npm = (dir: string, args: string[]): Result =>
    client.exec(["npm", ...args], { cwd: `/work/${dir}`, env: { [TOKEN_ENV]: stand.token } });
  return {
    client,
    stand,
    agent: `npm/${client.version}`,
    npm,
    must(dir, args) {
      const result = npm(dir, args);
      if (result.status !== 0) {
        throw new Error(`npm ${args.join(" ")} -> ${result.status}\n${tail(result.stderr)}`);
      }
      return result;
    },
    cache() {
      caches += 1;
      return `/tmp/npm-cache-${caches}`;
    },
  };
}

const step = (name: string, body: () => void): void => {
  try {
    body();
  } catch (error) {
    throw new Error(`${name}: ${error instanceof Error ? error.message : String(error)}`);
  }
};

const expectEqual = (what: string, got: string, want: string): void => {
  if (got !== want) {
    throw new Error(`${what}: ожидалось «${want}», получено «${got}»`);
  }
};

/** Пакет в каталоге `dir`: манифест, index.js с константой и .npmrc реестра. */
function writePackage(t: Tools, dir: string, name: string, version: string, rc: string): void {
  t.client.write(`${dir}/package.json`, packageJson(name, version));
  t.client.write(`${dir}/index.js`, `module.exports = "${CONSTANT}";\n`);
  t.client.write(`${dir}/.npmrc`, rc);
}

const hostedRc = npmrc(repoUrl(HOSTED_REPO), { scope: SCOPE, registry: repoUrl(HOSTED_REPO) });

/** Шаги 1-4: публикация, теги, deprecate. Возвращает события аудита. */
function publishAndMark(t: Tools, pkg: string): Expected[] {
  const event = (operation: "publish" | "tag" | "untag" | "deprecate", purl: string): Expected => ({
    kind: "api",
    agent: t.agent,
    operation,
    purl,
  });
  const events: Expected[] = [];
  step("1 публикация 1.0.0 и 1.1.0-beta.0 --tag next", () => {
    writePackage(t, "pub", pkg, FIRST, hostedRc);
    t.must("pub", ["publish"]);
    writePackage(t, "pub", pkg, BETA, hostedRc);
    t.must("pub", ["publish", "--tag", "next"]);
    events.push(event("publish", npmPurl(pkg, FIRST)), event("publish", npmPurl(pkg, BETA)));
  });
  step("2 dist-tags пакета: latest и next", () => {
    const raw = t.must("pub", ["view", pkg, "dist-tags", "--json"]).stdout;
    if (!sameTags(raw, { latest: FIRST, next: BETA })) {
      throw new Error(`dist-tags: ${raw.trim()}`);
    }
  });
  step("3 dist-tag add, ls, rm", () => {
    t.must("pub", ["dist-tag", "add", `${pkg}@${FIRST}`, "stable"]);
    events.push(event("tag", npmPurl(pkg, FIRST)));
    const listed = t.must("pub", ["dist-tag", "ls", pkg]).stdout;
    if (!listed.includes(`stable: ${FIRST}`)) {
      throw new Error(`dist-tag ls не показал stable\n${tail(listed)}`);
    }
    t.must("pub", ["dist-tag", "rm", pkg, "stable"]);
    events.push(event("untag", npmPurl(pkg)));
    const after = t.must("pub", ["dist-tag", "ls", pkg]).stdout;
    if (after.includes("stable")) {
      throw new Error(`stable остался после rm\n${tail(after)}`);
    }
  });
  step("4 deprecate", () => {
    t.must("pub", ["deprecate", `${pkg}@${FIRST}`, MESSAGE]);
    events.push(event("deprecate", npmPurl(pkg, FIRST)));
    const shown = t.must("pub", ["view", `${pkg}@${FIRST}`, "deprecated"]).stdout.trim();
    expectEqual("deprecated", shown, MESSAGE);
  });
  return events;
}

/** Шаг 5: установка из пустого кэша, предупреждение deprecate, require возвращает константу. */
function installConsumer(t: Tools, pkg: string): void {
  step("5 установка 1.0.0 из пустого кэша", () => {
    t.client.write("consumer/package.json", packageJson("consumer", FIRST));
    t.client.write("consumer/.npmrc", hostedRc);
    const installed = t.must("consumer", [
      "install",
      `${pkg}@${FIRST}`,
      "--no-audit",
      "--cache",
      t.cache(),
    ]);
    if (!hasDeprecationWarning(installed.stderr, `${pkg}@${FIRST}`, MESSAGE)) {
      throw new Error(`нет предупреждения о deprecate\n${tail(installed.stderr)}`);
    }
    const printed = t.client.exec(["node", "-e", `console.log(require("${pkg}"))`], {
      cwd: "/work/consumer",
    });
    expectEqual("require", printed.stdout.trim(), CONSTANT);
  });
}

/** Шаг 6: unpublish в окне, номер версии не переиспользуется. */
function unpublish(t: Tools, pkg: string): Expected[] {
  const events: Expected[] = [];
  step("6 unpublish 1.1.0-beta.0 и отказ повторной публикации", () => {
    t.must("pub", ["unpublish", `${pkg}@${BETA}`]);
    events.push({ kind: "api", agent: t.agent, operation: "delete", purl: npmPurl(pkg, BETA) });
    const listed = versionsOf(t.must("pub", ["view", pkg, "versions", "--json"]).stdout);
    if (listed.includes(BETA)) {
      throw new Error(`версия осталась в списке: ${listed.join(", ")}`);
    }
    const again = t.npm("pub", ["publish", "--tag", "next"]);
    if (again.status === 0) {
      throw new Error("повторная публикация tombstone прошла");
    }
    events.push({
      kind: "api",
      agent: t.agent,
      operation: "publish",
      purl: npmPurl(pkg, BETA),
      outcome: "Failure",
    });
    const conflict = accessLines(t.stand.craneLogs()).some(
      (line) => line.status === HTTP_CONFLICT && line.path.endsWith(`lib-${t.stand.id}`),
    );
    if (!conflict) {
      throw new Error("в access-журнале нет 409 на повторную публикацию");
    }
  });
  return events;
}

/** Установка пакета npmjs через proxy: свежий кэш и node_modules на каждый прогон. */
function proxyInstall(t: Tools): void {
  const rc = npmrc(repoUrl(npmProxyRepo));
  const install = (run: number, extra: string[]): void => {
    const dir = `proxy-${run}`;
    t.client.write(`${dir}/package.json`, packageJson(`proxyapp-${run}`, FIRST));
    t.client.write(`${dir}/.npmrc`, rc);
    t.must(dir, ["install", `${PROXY_PACKAGE}@${PROXY_VERSION}`, "--cache", t.cache(), ...extra]);
    const present = t.client.exec(["test", "-f", `node_modules/${PROXY_PACKAGE}/package.json`], {
      cwd: `/work/${dir}`,
    });
    if (present.status !== 0) {
      throw new Error(`прогон ${run}: ${PROXY_PACKAGE} не установлен`);
    }
  };
  const seen = (): number => accessLines(t.stand.craneLogs()).length;
  const first = seen();
  // Первый прогон с аудитом: ответ 501 на audit не должен ронять установку.
  install(1, []);
  const second = seen();
  install(2, ["--no-audit"]);
  const lines = accessLines(t.stand.craneLogs());
  const watched = [PROXY_PACKUMENT, PROXY_TARBALL];
  expectCache(watched, lines.slice(first, second), "miss");
  expectCache(watched, lines.slice(second), "hit");

  const egress = egressLines(t.stand.craneLogs());
  for (const path of [PROXY_PACKAGE, PROXY_TARBALL_PATH]) {
    const fetches = upstreamFetches(egress, path);
    if (fetches !== 1) {
      throw new Error(`выборок из upstream для ${path}: ${fetches}, ожидалась ровно одна`);
    }
  }
  t.client.write("proxy-view/.npmrc", rc);
  const tarball = t
    .must("proxy-view", [
      "view",
      `${PROXY_PACKAGE}@${PROXY_VERSION}`,
      "dist.tarball",
      "--cache",
      t.cache(),
    ])
    .stdout.trim();
  if (!tarball.startsWith(repoUrl(npmProxyRepo))) {
    throw new Error(`dist.tarball указывает мимо proxy: ${tarball}`);
  }
}

/** Выполняет сценарий npm и возвращает события аудита, которые он обязан породить. */
export function npmScenario(stand: Stand, client: NpmClient): Expected[] {
  const t = tools(stand, client);
  const pkg = `${SCOPE}/lib-${stand.id}`;
  const events = publishAndMark(t, pkg);
  installConsumer(t, pkg);
  events.push(...unpublish(t, pkg));
  step("7 proxy npmjs: промах, затем попадание", () => {
    proxyInstall(t);
  });
  return events;
}
