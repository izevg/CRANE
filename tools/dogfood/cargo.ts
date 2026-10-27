// Настоящий cargo в контейнере rust:<версия>: клиент ходит на https://localhost, как оператор
// на своей машине. Сценарий: публикация, индекс, сборка, yank, разрешение, unyank.
import type { Expected } from "./audit.ts";
import { type Client, startClient } from "./client.ts";
import { log, type Result, tail } from "./exec.ts";
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
const REGISTRY = "crane";
// Cargo не читает токен альтернативного реестра из окружения без явного провайдера учётных данных.
const CLIENT_ENV = {
  CARGO_HTTP_CAINFO: ROOT_CERT,
  CARGO_REGISTRIES_CRANE_INDEX: `sparse+${INDEX}`,
  CARGO_REGISTRIES_CRANE_CREDENTIAL_PROVIDER: "cargo:token",
  CARGO_TERM_COLOR: "never",
};

/**
 * Запускает клиент и ждёт, пока Caddy выпустит сертификат, а crane отдаст config.json.
 * `--retry-all-errors` повторяет и код 77 (корневого сертификата ещё нет), и отказ соединения.
 * Токен идёт через stdin, не через argv.
 */
export function startCargo(stand: Stand, image: string): CargoClient {
  log(`клиент ${image}: запуск`);
  const client = startClient(stand, image, CLIENT_ENV);
  try {
    const ready = client.exec(
      [
        "curl",
        "-fsS",
        "--retry",
        String(READY_SECONDS),
        "--retry-delay",
        "1",
        "--retry-all-errors",
        "--retry-max-time",
        String(READY_SECONDS),
        "--cacert",
        ROOT_CERT,
        "-K",
        "-",
        `${INDEX}config.json`,
      ],
      { input: `header = "Authorization: ${stand.token}"\n` },
    );
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
interface Crate {
  name: string;
  version: string;
  /** Строки секции [dependencies]. */
  deps?: string[];
  files: Record<string, string>;
}

const manifest = (crate: Crate): string =>
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

const step = (name: string, body: () => void): void => {
  try {
    body();
  } catch (error) {
    throw new Error(`${name}: ${error instanceof Error ? error.message : String(error)}`);
  }
};

/** Выполняет сценарий и возвращает события аудита, которые он обязан породить. */
export function scenario(stand: Stand, client: CargoClient, minor: number): Expected[] {
  const suffix = `${stand.id}_${minor}`;
  const a = `dfa_${suffix}`;
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
      env: { CARGO_HOME: home, CARGO_TARGET_DIR: `${home}-target`, [TOKEN_ENV]: token },
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
  const publish = (crate: Crate): void => {
    write(crate);
    must(crate.name, main, ["publish", "--registry", REGISTRY]);
    audited("publish", crate.name, crate.version);
  };

  step("1 публикация a@0.1.0, a@0.1.1, b@0.1.0", () => {
    publish(libA("0.1.0"));
    publish(libA("0.1.1"));
    publish({
      name: b,
      version: "0.1.0",
      deps: [dep(a, "0.1")],
      files: { "src/lib.rs": fn("a_version", `${a}::version()`) },
    });
  });

  step("2 поиск в индексе", () => {
    const info = must(".", main, ["info", a, "--registry", REGISTRY]);
    if (!`${info.stdout}${info.stderr}`.includes("0.1.1")) {
      throw new Error(`cargo info не показал 0.1.1\n${tail(info.stdout)}`);
    }
  });

  step("3 сборка app из b (транзитивно a@0.1.1)", () => {
    app("app", dep(b, "0.1"), `${b}::a_version()`);
    expectOutput(must("app", main, ["run", "-q"]), "0.1.1");
  });

  step("4 yank чужим токеном отклонён", () => {
    const denied = cargo(
      a,
      main,
      ["yank", "--registry", REGISTRY, "--version", "0.1.0", a],
      stand.wrongToken,
    );
    if (denied.status === 0) {
      throw new Error("yank с неверным токеном прошёл");
    }
    events.push({ kind: "authFailure", agent });
  });

  step("5 yank a@0.1.1", () => {
    must(a, main, ["yank", "--registry", REGISTRY, "--version", "0.1.1", a]);
    audited("yank", a, "0.1.1");
  });

  step("6 разрешение обходит yanked: app2 получает a@0.1.0", () => {
    app("app2", dep(b, "0.1"), `${b}::a_version()`);
    expectOutput(must("app2", freshHome(), ["run", "-q"]), "0.1.0");
  });

  step("7 прямая зависимость от yanked версии не разрешается", () => {
    app("app3", dep(a, "=0.1.1"), `${a}::version()`);
    const locked = cargo("app3", freshHome(), ["generate-lockfile"]);
    if (locked.status === 0 || !YANKED.test(locked.stderr)) {
      throw new Error(`generate-lockfile -> ${locked.status}\n${tail(locked.stderr)}`);
    }
  });

  step("8 Cargo.lock с yanked версией собирается из пустого кэша", () => {
    expectOutput(must("app", freshHome(), ["run", "-q", "--locked"]), "0.1.1");
  });

  step("9 unyank a@0.1.1", () => {
    must(a, main, ["yank", "--registry", REGISTRY, "--undo", "--version", "0.1.1", a]);
    audited("unyank", a, "0.1.1");
    app("app4", dep(b, "0.1"), `${b}::a_version()`);
    expectOutput(must("app4", freshHome(), ["run", "-q"]), "0.1.1");
  });

  return events;
}
