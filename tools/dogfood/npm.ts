// Настоящий npm в контейнере node:current: клиент ходит на https://localhost, как оператор на своей машине.
// Hosted принимает только имена с областью (`@dogfood/..`); форма без области покрыта proxy.
// Здесь клиент (образ, запуск, .npmrc) и разбор вывода npm; сценарии hosted и proxy лежат в npm.scenario.ts.
// Токен идёт только через окружение docker CLI.
import type { Egress } from "./audit.ts";
import { authorizedCurl, ROOT_CERT } from "./cargo.ts";
import { type Client, startClient } from "./client.ts";
import { docker, log, MS_PER_SECOND, run, SECONDS_PER_MINUTE, tail } from "./exec.ts";
import { npmProxyRepo, type Stand } from "./stand.ts";

export interface NpmClient extends Client {
  /** Версия npm после `npm install -g npm@latest`. */
  version: string;
}

const BASE_IMAGE = "node:current-bookworm-slim";
const BUILD_TIMEOUT_MINUTES = 10;
const BUILD_TIMEOUT_MS = BUILD_TIMEOUT_MINUTES * SECONDS_PER_MINUTE * MS_PER_SECOND;
const READY_SECONDS = 60;
export const TOKEN_ENV = "NPM_TOKEN";
export const HOSTED_REPO = "internal";
const HOST = "https://localhost";
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
export const repoUrl = (repo: string): string => `${HOST}/npm/${repo}/`;

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
