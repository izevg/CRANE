// Одноразовый стенд: тот же docker-compose.yaml, что у оператора, с отдельным проектом, секретами,
// подсетью и портами. Всё, что создано, снимается в down(), в том числе после сбоя.
import { randomBytes } from "node:crypto";
import { chmodSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { docker, log, MS_PER_SECOND, run, SECONDS_PER_MINUTE, warn } from "./exec.ts";

export interface Stand {
  /** Случайный идентификатор прогона: суффикс имён проекта, образа и крейтов. */
  id: string;
  project: string;
  /** Детерминированное имя контейнера Caddy в compose: `<проект>-caddy-1`. */
  caddy: string;
  /** Имя контейнера crane в compose: `<проект>-crane-1`; в его сетевом пространстве гасится маршрут. */
  crane: string;
  principal: string;
  token: string;
  wrongToken: string;
  /** Секреты стенда для проверки утечек и редактирования сохранённых журналов. */
  secrets: { name: string; value: string }[];
  /** Секреты, сборка образа, compose up. Побочные эффекты начинаются здесь. */
  start(): void;
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
const SECRET_MODE = 0o644;
const DIR_MODE = 0o700;
const ENV_LINE = /^([A-Z][A-Z0-9_]*)=(.*)$/;
const OVERLAP = /Pool overlaps|invalid pool request/;
const ENV_DIST = ".env.dist";
/**
 * Имя proxy-репозитория crates.io для цели cargo с номером `index`: `/cargo/<имя>/`. Ключ кэша включает
 * репозиторий, поэтому у каждой цели свой кэш и первый прогон каждой честно идёт мимо кэша.
 */
export const proxyRepo = (index: number): string => `crates-io-${index + 1}`;

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
  const token = hex(HEX_BYTES_SECRET);
  const wrongToken = hex(HEX_BYTES_SECRET);
  const postgresPassword = hex(HEX_BYTES_SECRET);
  // Пользователь и база берутся из .env.dist: URL в секрете обязан совпасть с тем, что создаёт postgres.
  const { user, database } = postgresNames(join(root, ENV_DIST));
  const databaseUrl = `postgres://${user}:${postgresPassword}@postgres:5432/${database}`;
  const principal = `dogfood-${id}`;
  const secrets = [
    { name: "токен", value: token },
    { name: "неверный токен", value: wrongToken },
    { name: "пароль PostgreSQL", value: postgresPassword },
    { name: "URL каталога", value: databaseUrl },
  ];

  const overrides = join(tmp, "overrides.env");
  const compose = ["compose", "-p", project, "--env-file", ENV_DIST, "--env-file", overrides];
  const composeRun = (args: string[]): string => docker([...compose, ...args], { cwd: root });
  const logs = (services: string[]): string =>
    composeRun(["logs", "--no-log-prefix", "--no-color", ...services]);

  return {
    id,
    project,
    caddy: `${project}-caddy-1`,
    crane: `${project}-crane-1`,
    principal,
    token,
    wrongToken,
    secrets,
    start() {
      mkdirSync(tmp, { mode: DIR_MODE });
      writeSecrets(tmp, { token, postgresPassword, databaseUrl });
      log(`стенд ${project}: сборка образа ${image}`);
      docker(["build", "-t", image, root], { timeoutMs: BUILD_TIMEOUT_MS });
      startCompose(tmp, overrides, { id, principal, proxyRepos }, composeRun);
    },
    craneLogs: () => logs(["crane"]),
    allLogs: () => logs([]),
    saveLogs(path) {
      let content = logs([]);
      for (const secret of secrets) {
        content = content.split(secret.value).join(`<${secret.name}>`);
      }
      mkdirSync(dirname(path), { recursive: true });
      writeFileSync(path, content);
    },
    down() {
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
    },
  };
}

/** Файлы 0644 в каталоге 0700: на Linux bind-монтируемый секрет читает uid 65532, а не владелец хоста. */
function writeSecrets(
  tmp: string,
  values: { token: string; postgresPassword: string; databaseUrl: string },
): void {
  chmodSync(tmp, DIR_MODE);
  const files: [string, string][] = [
    ["crane_token", values.token],
    ["postgres_password", values.postgresPassword],
    ["database_url", values.databaseUrl],
  ];
  for (const [name, value] of files) {
    writeFileSync(join(tmp, name), value);
    chmodSync(join(tmp, name), SECRET_MODE);
  }
}

function startCompose(
  tmp: string,
  overrides: string,
  settings: { id: string; principal: string; proxyRepos: string[] },
  composeRun: (args: string[]) => string,
): void {
  for (let attempt = 1; ; attempt += 1) {
    const subnet = randomSubnet();
    // Только отличия от .env.dist. Порты caddy: loopback и свободный порт хоста ("127.0.0.1::80").
    const lines = [
      "CRANE_IMAGE=crane-dogfood",
      `CRANE_VERSION=${settings.id}`,
      `CRANE_TOKEN_SECRET=${join(tmp, "crane_token")}`,
      `POSTGRES_PASSWORD_SECRET=${join(tmp, "postgres_password")}`,
      `CRANE_DATABASE_URL_SECRET=${join(tmp, "database_url")}`,
      `CRANE_TOKEN_PRINCIPAL=${settings.principal}`,
      `CRANE_CARGO_PROXY_REPOS=${settings.proxyRepos.join(",")}`,
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

/** POSTGRES_USER и POSTGRES_DB из .env.dist; отсутствие любого из них — ошибка. */
function postgresNames(path: string): { user: string; database: string } {
  const values = new Map<string, string>();
  for (const line of readFileSync(path, "utf8").split("\n")) {
    const match = ENV_LINE.exec(line);
    if (match?.[1] !== undefined && match[2] !== undefined) {
      values.set(match[1], match[2]);
    }
  }
  const user = values.get("POSTGRES_USER");
  const database = values.get("POSTGRES_DB");
  if (!(user && database)) {
    throw new Error(`${path}: нужны POSTGRES_USER и POSTGRES_DB`);
  }
  return { user, database };
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
