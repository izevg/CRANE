// Одноразовый стенд: те же docker-compose.yaml и docker-compose.s3.yaml, что у оператора, с отдельным проектом, секретами,
// подсетью и портами. Всё, что создано, снимается в down(), в том числе после сбоя.
import { randomBytes } from "node:crypto";
import { chmodSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { initSecrets } from "../secrets-init.ts";
import { docker, log, MS_PER_SECOND, run, SECONDS_PER_MINUTE, warn } from "./exec.ts";

export interface Stand {
  /** Случайный идентификатор прогона: суффикс имён проекта, образа и крейтов. */
  id: string;
  project: string;
  /** Детерминированное имя контейнера Caddy в compose: `<проект>-caddy-1`. */
  caddy: string;
  /** Имя контейнера SeaweedFS в compose: `<проект>-seaweedfs-1`. */
  seaweedfs: string;
  /** Бакет CAS (S3_BUCKET из .env.dist), он же в CRANE_BLOBS. */
  bucket: string;
  /** Имя контейнера crane в compose: `<проект>-crane-1`; в его сетевом пространстве гасится маршрут. */
  crane: string;
  principal: string;
  token: string;
  wrongToken: string;
  /** Секреты стенда для проверки утечек и редактирования сохранённых журналов. */
  secrets: { name: string; value: string }[];
  /** Секреты, сборка образа, compose up. Побочные эффекты начинаются здесь. */
  start(): void;
  /**
   * Перезапускает crane и возвращается, когда он снова `healthy`. Обнуляет память процесса (L1), контейнер
   * и журнал остаются прежними. Сетевое пространство пересоздаётся, поэтому снятые маршруты возвращаются.
   */
  restartCrane(): void;
  /** Журнал сервиса crane (NDJSON) без префиксов compose. */
  craneLogs(): string;
  allLogs(): string;
  saveLogs(path: string): void;
  /** Идемпотентно, не бросает, безопасно после частичного start; возвращает оставшиеся ресурсы. */
  down(): string[];
}

const SUBNET_ATTEMPTS = 8;
const HEX_BYTES_SECRET = 32;
const HEX_BYTES_ID = 4;
const BUILD_TIMEOUT_MINUTES = 45;
const BUILD_TIMEOUT_MS = BUILD_TIMEOUT_MINUTES * SECONDS_PER_MINUTE * MS_PER_SECOND;
const UP_TIMEOUT_SECONDS = 240;
const DOWN_TIMEOUT_SECONDS = 5;
// Второй октет подсети: 10.200–10.254, третий — любой.
const SUBNET_SECOND_MIN = 200;
const SUBNET_SECOND_SPAN = 55;
const OCTET_VALUES = 256;
const DIR_MODE = 0o700;
const ENV_LINE = /^([A-Z][A-Z0-9_]*)=(.*)$/;
const OVERLAP = /Pool overlaps|invalid pool request/;
const ENV_DIST = ".env.dist";
// Переменные с путями секретов → имя файла в каталоге стенда. Имя файла служит именем секрета в
// сообщениях проверки утечек. Форматы файлов (URL каталога, ключи S3) знает только secrets-init.
const SECRET_FILES = {
  CRANE_TOKEN_SECRET: "crane_token",
  POSTGRES_PASSWORD_SECRET: "postgres_password",
  CRANE_DATABASE_URL_SECRET: "database_url",
  S3_ACCESS_KEY_ID_SECRET: "s3_access_key_id",
  S3_SECRET_ACCESS_KEY_SECRET: "s3_secret_access_key",
} as const;
const WRONG_TOKEN_NAME = "неверный токен";
const S3_ENDPOINT = "http://seaweedfs:8333";
// Стенд = Valkey + S3 + PostgreSQL: те же файлы и профиль, что включает оператор.
const COMPOSE_MODES = [
  "-f",
  "docker-compose.yaml",
  "-f",
  "docker-compose.s3.yaml",
  "--profile",
  "cache",
];
/**
 * Имя proxy-репозитория crates.io для цели cargo с номером `index`: `/cargo/<имя>/`. Ключ кэша включает
 * репозиторий, поэтому у каждой цели свой кэш и первый прогон каждой честно идёт мимо кэша.
 */
export const proxyRepo = (index: number): string => `crates-io-${index + 1}`;
/** Proxy-репозиторий npmjs для npm-сценария: `/npm/<имя>/`; своё имя, чтобы не сдвигать индексы `proxyRepo`. */
export const npmProxyRepo = "npmjs-1";

const hex = (bytes: number): string => randomBytes(bytes).toString("hex");

/** Случайная /24 из 10.200.0.0–10.254.255.0: вряд ли пересечётся с сетями хоста и соседних проектов. */
function randomSubnet(): string {
  const second = SUBNET_SECOND_MIN + (randomBytes(1).readUInt8(0) % SUBNET_SECOND_SPAN);
  const third = randomBytes(1).readUInt8(0) % OCTET_VALUES;
  return `10.${second}.${third}.0/24`;
}

/** Описание стенда без побочных эффектов: имена, секреты, команды. Запуск и уборка — снаружи. */
export function describeStand(root: string, proxyRepos: string[]): Stand {
  const id = hex(HEX_BYTES_ID);
  const project = `crane-dogfood-${id}`;
  const image = `crane-dogfood:${id}`;
  const tmp = join(tmpdir(), project);
  const wrongToken = hex(HEX_BYTES_SECRET);
  // Пользователь и база берутся из .env.dist: URL в секрете обязан совпасть с тем, что создаёт postgres.
  const names = envDistValues(join(root, ENV_DIST));
  const principal = `dogfood-${id}`;
  // Секреты создаёт secrets-init в start(); до него токена и списка нет.
  let token = "";
  const secrets: { name: string; value: string }[] = [
    { name: WRONG_TOKEN_NAME, value: wrongToken },
  ];
  const secretPaths = Object.fromEntries(
    Object.entries(SECRET_FILES).map(([variable, file]) => [variable, join(tmp, file)]),
  );

  const overrides = join(tmp, "overrides.env");
  const compose = composeCommand(project, overrides);
  const composeRun = (args: string[]): string => docker([...compose, ...args], { cwd: root });
  const logs = (services: string[]): string =>
    composeRun(["logs", "--no-log-prefix", "--no-color", ...services]);

  return {
    id,
    project,
    caddy: `${project}-caddy-1`,
    seaweedfs: `${project}-seaweedfs-1`,
    bucket: names.bucket,
    crane: `${project}-crane-1`,
    principal,
    get token() {
      return token;
    },
    wrongToken,
    secrets,
    start() {
      mkdirSync(tmp, { mode: DIR_MODE });
      // mkdir уважает umask, а каталог обязан быть ровно 0700 (файлы внутри 0644 делает secrets-init).
      chmodSync(tmp, DIR_MODE);
      initSecrets({ ...secretPaths, POSTGRES_USER: names.user, POSTGRES_DB: names.database }, tmp);
      token = readSecret(secretPaths, "CRANE_TOKEN_SECRET");
      for (const [variable, file] of Object.entries(SECRET_FILES)) {
        secrets.push({ name: file, value: readSecret(secretPaths, variable) });
      }
      log(`стенд ${project}: сборка образа ${image}`);
      docker(["build", "-t", image, root], { timeoutMs: BUILD_TIMEOUT_MS });
      startCompose(
        overrides,
        {
          id,
          principal,
          proxyRepos,
          blobs: `s3://${names.bucket}?endpoint=${S3_ENDPOINT}&allow_http=true`,
        },
        secretPaths,
        composeRun,
      );
    },
    restartCrane: () => restartCrane(composeRun),
    craneLogs: () => logs(["crane"]),
    allLogs: () => logs([]),
    saveLogs(path) {
      const content = redact(logs([]), secrets);
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, content);
    },
    down: () => removeStand({ project, image, tmp }, composeRun),
  };
}

/** Базовая команда compose: режимы в ней, чтобы up, logs и down (в том числе повторный после смены подсети) видели valkey, seaweedfs, сеть и том s3. */
function composeCommand(project: string, overrides: string): string[] {
  return [
    "compose",
    "-p",
    project,
    ...COMPOSE_MODES,
    "--env-file",
    ENV_DIST,
    "--env-file",
    overrides,
  ];
}

function restartCrane(composeRun: (args: string[]) => string): void {
  // `restart`, а не пересоздание: контейнер тот же, журнал compose сохраняется, а память процесса
  // обнулена. Клиент не делает запросов до `healthy`: пока crane не готов, Caddy отвечает 502.
  composeRun(["restart", "--timeout", String(DOWN_TIMEOUT_SECONDS), "crane"]);
  // После restart docker сбрасывает статус healthcheck в `starting`, поэтому `--wait` ждёт новую проверку.
  // `--no-recreate --no-deps` не дают up тронуть контейнер и соседние сервисы.
  composeRun([
    "up",
    "-d",
    "--wait",
    "--wait-timeout",
    String(UP_TIMEOUT_SECONDS),
    "--no-recreate",
    "--no-deps",
    "crane",
  ]);
}

/** Журнал без секретов: значение каждого заменено на `<имя>`. */
function redact(content: string, secrets: { name: string; value: string }[]): string {
  return secrets.reduce((acc, secret) => acc.split(secret.value).join(`<${secret.name}>`), content);
}

/** Уборка стенда: каждый шаг независим, сбой одного не мешает остальным. Возвращает то, что осталось. */
function removeStand(
  target: { project: string; image: string; tmp: string },
  composeRun: (args: string[]) => string,
): string[] {
  const { project, image, tmp } = target;
  const attempt = (what: string, action: () => void): void => {
    try {
      action();
    } catch (error) {
      warn(`down: ${what}: ${error instanceof Error ? error.message : String(error)}`);
    }
  };
  attempt("клиентские контейнеры", () => {
    const ids = docker(["ps", "-aq", "--filter", `label=crane-dogfood=${project}`]).split("\n");
    const present = ids.filter((line) => line.trim() !== "");
    if (present.length > 0) {
      docker(["rm", "-f", ...present]);
    }
  });
  attempt("compose down", () => {
    composeRun(["down", "-v", "--remove-orphans", "--timeout", String(DOWN_TIMEOUT_SECONDS)]);
  });
  attempt("образ", () => {
    run("docker", ["image", "rm", image]);
  });
  attempt("временный каталог", () => {
    rmSync(tmp, { recursive: true, force: true });
  });
  return leftovers(project, image);
}

function readSecret(paths: Record<string, string>, variable: string): string {
  return readFileSync(paths[variable] ?? "", "utf8").trim();
}

function startCompose(
  overrides: string,
  settings: { id: string; principal: string; proxyRepos: string[]; blobs: string },
  secretPaths: Record<string, string>,
  composeRun: (args: string[]) => string,
): void {
  for (let attempt = 1; ; attempt += 1) {
    const subnet = randomSubnet();
    // Только отличия от .env.dist. Порты caddy: loopback и свободный порт хоста ("127.0.0.1::80").
    const lines = [
      "CRANE_IMAGE=crane-dogfood",
      `CRANE_VERSION=${settings.id}`,
      ...Object.entries(secretPaths).map(([variable, path]) => `${variable}=${path}`),
      // CAS на S3: бакет создаёт seaweedfs-init, ключи лежат в файлах секретов, в URL их нет.
      `CRANE_BLOBS=${settings.blobs}`,
      `CRANE_TOKEN_PRINCIPAL=${settings.principal}`,
      `CRANE_CARGO_PROXY_REPOS=${settings.proxyRepos.join(",")}`,
      // L2-кэш в Valkey. Адрес без пароля и учётных данных, поэтому не секрет: сервис valkey в compose
      // без auth и доступен только по сети default.
      "CRANE_CACHE=valkey://valkey:6379",
      "CRANE_NPM_REPOS=internal",
      `CRANE_NPM_PROXY_REPOS=${npmProxyRepo}`,
      `CRANE_EDGE_SUBNET=${subnet}`,
      "CADDY_HTTP_PORT=127.0.0.1:",
      "CADDY_HTTPS_PORT=127.0.0.1:",
    ];
    writeFileSync(overrides, `${lines.join("\n")}\n`);
    try {
      log(`compose up (подсеть ${subnet})`);
      composeRun(["up", "-d", "--wait", "--wait-timeout", String(UP_TIMEOUT_SECONDS)]);
      return;
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (attempt >= SUBNET_ATTEMPTS || !OVERLAP.test(message)) {
        throw error;
      }
      warn(`подсеть ${subnet} занята, пробую другую`);
      composeRun(["down", "-v", "--remove-orphans", "--timeout", String(DOWN_TIMEOUT_SECONDS)]);
    }
  }
}

/** POSTGRES_USER, POSTGRES_DB и S3_BUCKET из .env.dist; отсутствие любого из них — ошибка. */
function envDistValues(path: string): { user: string; database: string; bucket: string } {
  const values = new Map<string, string>();
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const match = ENV_LINE.exec(line);
    if (match?.[1] !== undefined && match[2] !== undefined) {
      values.set(match[1], match[2]);
    }
  }
  const user = values.get("POSTGRES_USER");
  const database = values.get("POSTGRES_DB");
  const bucket = values.get("S3_BUCKET");
  if (!(user && database && bucket)) {
    throw new Error(`${path}: нужны POSTGRES_USER, POSTGRES_DB и S3_BUCKET`);
  }
  return { user, database, bucket };
}

/** Всё, что относится к стенду и ещё существует. */
function leftovers(project: string, image: string): string[] {
  const byProject = ["--filter", `label=com.docker.compose.project=${project}`];
  const list = (args: string[]): string[] =>
    run("docker", args)
      .stdout.split("\n")
      .filter((line) => line.trim() !== "");
  return [
    ...list(["ps", "-a", ...byProject, "--format", "container {{.Names}}"]),
    ...list([
      "ps",
      "-a",
      "--filter",
      `label=crane-dogfood=${project}`,
      "--format",
      "container {{.Names}}",
    ]),
    ...list(["volume", "ls", ...byProject, "--format", "volume {{.Name}}"]),
    ...list(["network", "ls", ...byProject, "--format", "network {{.Name}}"]),
    ...list(["image", "ls", "--format", "image {{.Repository}}:{{.Tag}}", image]),
  ];
}
