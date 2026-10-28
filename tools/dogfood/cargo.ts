// Настоящий cargo в контейнере rust:<версия>: клиент ходит на https://localhost, как оператор
// на своей машине. Сценарий: публикация, индекс, сборка, yank, разрешение, unyank.
import type { Expected } from "./audit.ts";
import { type Client, startClient } from "./client.ts";
import { log, type Result, step, tail } from "./exec.ts";
import type { Stand } from "./stand.ts";
import { parseCargo } from "./versions.ts";

export interface CargoClient extends Client {
  /** Полная версия, как её сообщает cargo: 1.M.p. */
  version: string;
}

export const ROOT_CERT = "/caddy/caddy/pki/authorities/local/root.crt";
const INDEX = "https://localhost/cargo/internal/index/";
const READY_SECONDS = 60;
const YANKED = /yanked/i;
const TOKEN_ENV = "CARGO_REGISTRIES_CRANE_TOKEN";
/** Имя hosted-реестра в конфигурации клиента: `[registries.crane]` приходит из env. */
export const REGISTRY = "crane";
// Cargo не читает токен альтернативного реестра из окружения без явного провайдера учётных данных.
const CLIENT_ENV = {
  CARGO_HTTP_CAINFO: ROOT_CERT,
  CARGO_REGISTRIES_CRANE_INDEX: `sparse+${INDEX}`,
  CARGO_REGISTRIES_CRANE_CREDENTIAL_PROVIDER: "cargo:token",
  CARGO_TERM_COLOR: "never",
};

/** Токен hosted-реестра в окружение cargo: значение идёт через env, в argv попадает только имя. */
export const craneCredentials = (token: string): Record<string, string> => ({
  [TOKEN_ENV]: token,
});

/** Окружение cargo в контейнере: свой CARGO_HOME и CARGO_TARGET_DIR рядом, токен hosted-реестра. */
export const cargoEnv = (home: string, token: string): Record<string, string> => ({
  CARGO_HOME: home,
  CARGO_TARGET_DIR: `${home}-target`,
  ...craneCredentials(token),
});

/**
 * curl с токеном через stdin (`-K -`): секрет не виден в `ps`. `args` — всё остальное, включая URL.
 * `scheme` — схема заголовка (npm: `Bearer`); без неё токен идёт как есть, как у cargo.
 */
export const authorizedCurl = (
  client: Client,
  token: string,
  args: string[],
  scheme?: string,
): Result =>
  client.exec(["curl", "--cacert", ROOT_CERT, "-K", "-", ...args], {
    input: `header = "Authorization: ${scheme === undefined ? "" : `${scheme} `}${token}"\n`,
  });

/**
 * Запускает клиент и ждёт, пока Caddy выпустит сертификат, а crane отдаст config.json.
 * `--retry-all-errors` повторяет и код 77 (корневого сертификата ещё нет), и отказ соединения.
 * Токен идёт через stdin, не через argv.
 */
export function startCargo(stand: Stand, image: string): CargoClient {
  log(`клиент ${image}: запуск`);
  const client = startClient(stand, image, CLIENT_ENV);
  try {
    const ready = authorizedCurl(client, stand.token, [
      "-fsS",
      "--retry",
      String(READY_SECONDS),
      "--retry-delay",
      "1",
      "--retry-all-errors",
      "--retry-max-time",
      String(READY_SECONDS),
      `${INDEX}config.json`,
    ]);
    if (ready.status !== 0) {
      throw new Error(`индекс не ответил за ${READY_SECONDS} с: ${tail(ready.stderr)}`);
    }
    const { version } = parseCargo(client.exec(["cargo", "--version"]).stdout);
    return { ...client, version };
  } catch (error) {
    client.stop();
    throw error;
  }
}

/** Крейт данными: манифест плюс файлы относительно его каталога. */
export interface Crate {
  name: string;
  version: string;
  /** Строки секции [dependencies]. */
  deps?: string[];
  files: Record<string, string>;
}

export const manifest = (crate: Crate): string =>
  [
    "[package]",
    `name = "${crate.name}"`,
    `version = "${crate.version}"`,
    'edition = "2021"',
    'description = "CRANE dogfood"',
    'license = "MIT"',
    "",
    "[dependencies]",
    ...(crate.deps ?? []),
    "",
  ].join("\n");

const dep = (name: string, requirement: string): string =>
  `${name} = { version = "${requirement}", registry = "${REGISTRY}" }`;

const fn = (name: string, body: string): string =>
  `pub fn ${name}() -> &'static str {\n    ${body}\n}\n`;

/** Имя первого крейта сценария: по нему же проверяется, что hosted побеждает proxy. */
export const hostedName = (stand: Stand, minor: number): string => `dfa_${stand.id}_${minor}`;

/** Всё, что нужно шагам сценария: запуск cargo, запись крейтов и учёт ожидаемых событий аудита. */
interface Kit {
  agent: string;
  a: string;
  b: string;
  main: string;
  events: Expected[];
  freshHome(): string;
  cargo(dir: string, home: string, args: string[], token?: string): Result;
  must(dir: string, home: string, args: string[]): Result;
  expectOutput(result: Result, want: string): void;
  app(name: string, dependency: string, call: string): void;
  audited(operation: "publish" | "yank" | "unyank", name: string, version: string): void;
  libA(version: string): Crate;
  publish(crate: Crate): void;
}

function scenarioKit(stand: Stand, client: CargoClient, minor: number): Kit {
  const suffix = `${stand.id}_${minor}`;
  const a = hostedName(stand, minor);
  const b = `dfb_${suffix}`;
  const agent = `cargo/${client.version}`;
  const main = "/tmp/h-main";
  const events: Expected[] = [];
  let homes = 0;

  const freshHome = (): string => {
    homes += 1;
    return `/tmp/h-${minor}-${homes}`;
  };
  const cargo = (dir: string, home: string, args: string[], token = stand.token): Result =>
    client.exec(["cargo", ...args], {
      cwd: `/work/${dir}`,
      env: cargoEnv(home, token),
    });
  const must = (dir: string, home: string, args: string[]): Result => {
    const result = cargo(dir, home, args);
    if (result.status !== 0) {
      throw new Error(`cargo ${args.join(" ")} -> ${result.status}\n${tail(result.stderr)}`);
    }
    return result;
  };
  const expectOutput = (result: Result, want: string): void => {
    const got = result.stdout.trim();
    if (got !== want) {
      throw new Error(`ожидалось «${want}», получено «${got}»\n${tail(result.stderr)}`);
    }
  };
  const write = (crate: Crate): void => {
    client.write(`${crate.name}/Cargo.toml`, manifest(crate));
    for (const [path, text] of Object.entries(crate.files)) {
      client.write(`${crate.name}/${path}`, text);
    }
  };
  const app = (name: string, dependency: string, call: string): void => {
    write({
      name,
      version: "0.1.0",
      deps: [dependency],
      files: { "src/main.rs": `fn main() {\n    println!("{}", ${call});\n}\n` },
    });
  };
  const audited = (operation: "publish" | "yank" | "unyank", name: string, version: string) => {
    events.push({ kind: "api", agent, operation, purl: `pkg:cargo/${name}@${version}` });
  };
  const libA = (version: string): Crate => ({
    name: a,
    version,
    files: { "src/lib.rs": fn("version", `"${version}"`) },
  });
  const published = new Set<string>();
  const publish = (crate: Crate): void => {
    write(crate);
    must(crate.name, main, ["publish", "--registry", REGISTRY]);
    // Первая публикация нового имени (его нет ни у upstream, ни в hosted) становится его резервированием.
    if (!published.has(crate.name)) {
      published.add(crate.name);
      events.push({ kind: "reserve", agent });
    }
    audited("publish", crate.name, crate.version);
  };

  return {
    agent,
    a,
    b,
    main,
    events,
    freshHome,
    cargo,
    must,
    expectOutput,
    app,
    audited,
    libA,
    publish,
  };
}

/** Шаги 1-3: публикация трёх версий, поиск в индексе, сборка app из транзитивной зависимости. */
function publishAndBuild(k: Kit): void {
  step("1 публикация a@0.1.0, a@0.1.1, b@0.1.0", () => {
    k.publish(k.libA("0.1.0"));
    k.publish(k.libA("0.1.1"));
    k.publish({
      name: k.b,
      version: "0.1.0",
      deps: [dep(k.a, "0.1")],
      files: { "src/lib.rs": fn("a_version", `${k.a}::version()`) },
    });
  });

  step("2 поиск в индексе", () => {
    const info = k.must(".", k.main, ["info", k.a, "--registry", REGISTRY]);
    if (!`${info.stdout}${info.stderr}`.includes("0.1.1")) {
      throw new Error(`cargo info не показал 0.1.1\n${tail(info.stdout)}`);
    }
  });

  step("3 сборка app из b (транзитивно a@0.1.1)", () => {
    k.app("app", dep(k.b, "0.1"), `${k.b}::a_version()`);
    k.expectOutput(k.must("app", k.main, ["run", "-q"]), "0.1.1");
  });
}

/** Шаги 4-8: yank чужим токеном и своим, как он виден разрешению зависимостей и Cargo.lock. */
function yankFlow(k: Kit, stand: Stand): void {
  step("4 yank чужим токеном отклонён", () => {
    const denied = k.cargo(
      k.a,
      k.main,
      ["yank", "--registry", REGISTRY, "--version", "0.1.0", k.a],
      stand.wrongToken,
    );
    if (denied.status === 0) {
      throw new Error("yank с неверным токеном прошёл");
    }
    k.events.push({ kind: "authFailure", agent: k.agent });
  });

  step("5 yank a@0.1.1", () => {
    k.must(k.a, k.main, ["yank", "--registry", REGISTRY, "--version", "0.1.1", k.a]);
    k.audited("yank", k.a, "0.1.1");
  });

  step("6 разрешение обходит yanked: app2 получает a@0.1.0", () => {
    k.app("app2", dep(k.b, "0.1"), `${k.b}::a_version()`);
    k.expectOutput(k.must("app2", k.freshHome(), ["run", "-q"]), "0.1.0");
  });

  step("7 прямая зависимость от yanked версии не разрешается", () => {
    k.app("app3", dep(k.a, "=0.1.1"), `${k.a}::version()`);
    const locked = k.cargo("app3", k.freshHome(), ["generate-lockfile"]);
    if (locked.status === 0 || !YANKED.test(locked.stderr)) {
      throw new Error(`generate-lockfile -> ${locked.status}\n${tail(locked.stderr)}`);
    }
  });

  step("8 Cargo.lock с yanked версией собирается из пустого кэша", () => {
    k.expectOutput(k.must("app", k.freshHome(), ["run", "-q", "--locked"]), "0.1.1");
  });
}

/** Шаг 9: unyank возвращает версию в разрешение. */
function unyank(k: Kit): void {
  step("9 unyank a@0.1.1", () => {
    k.must(k.a, k.main, ["yank", "--registry", REGISTRY, "--undo", "--version", "0.1.1", k.a]);
    k.audited("unyank", k.a, "0.1.1");
    k.app("app4", dep(k.b, "0.1"), `${k.b}::a_version()`);
    k.expectOutput(k.must("app4", k.freshHome(), ["run", "-q"]), "0.1.1");
  });
}

/** Выполняет сценарий и возвращает события аудита, которые он обязан породить. */
export function scenario(stand: Stand, client: CargoClient, minor: number): Expected[] {
  const k = scenarioKit(stand, client, minor);
  publishAndBuild(k);
  yankFlow(k, stand);
  unyank(k);
  return k.events;
}
