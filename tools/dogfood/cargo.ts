// Настоящий cargo в контейнере rust:<версия>: клиент делит сетевое пространство имён с Caddy,
// поэтому ходит на https://localhost, как оператор на своей машине.
import type { Expected } from "./audit.ts";
import { docker, log, MS_PER_SECOND, type Result, run, runOk, sleep, tail } from "./exec.ts";
import type { Stand } from "./stand.ts";
import { parseCargoVersion } from "./versions.ts";

export interface Client {
  container: string;
  /** Образ rust:<версия>-bookworm. */
  image: string;
  /** Полная версия, как её сообщает cargo: 1.M.p. */
  version: string;
  stop(): void;
}

const ROOT_CERT = "/caddy/caddy/pki/authorities/local/root.crt";
const INDEX = "https://localhost/cargo/internal/index/";
const READY_TIMEOUT_SECONDS = 60;
const READY_TIMEOUT_MS = READY_TIMEOUT_SECONDS * MS_PER_SECOND;
const READY_PAUSE_MS = MS_PER_SECOND;
const YANKED = /yanked/i;
const TOKEN_ENV = "CARGO_REGISTRIES_CRANE_TOKEN";
const REGISTRY = "crane";

/** Ждёт, пока Caddy выпустит сертификат, а crane отдаст config.json. Токен идёт через stdin, не через argv. */
function waitReady(stand: Stand, container: string): void {
  const deadline = Date.now() + READY_TIMEOUT_MS;
  let last = "";
  while (Date.now() < deadline) {
    const result = run(
      "docker",
      [
        "exec",
        "-i",
        container,
        "curl",
        "-fsS",
        "--cacert",
        ROOT_CERT,
        "-K",
        "-",
        `${INDEX}config.json`,
      ],
      { input: `header = "Authorization: ${stand.token}"\n` },
    );
    if (result.status === 0) {
      return;
    }
    last = tail(result.stderr);
    sleep(READY_PAUSE_MS);
  }
  throw new Error(`индекс не ответил за ${READY_TIMEOUT_SECONDS} с: ${last}`);
}

export function startClient(stand: Stand, image: string): Client {
  log(`клиент ${image}: запуск`);
  const container = runOk(
    "docker",
    [
      "run",
      "-d",
      "--rm",
      "--label",
      `crane-dogfood=${stand.project}`,
      "--network",
      `container:${stand.caddyId}`,
      "-v",
      `${stand.project}_caddy-data:/caddy:ro`,
      "-w",
      "/work",
      "-e",
      `CARGO_HTTP_CAINFO=${ROOT_CERT}`,
      "-e",
      `CARGO_REGISTRIES_CRANE_INDEX=sparse+${INDEX}`,
      "-e",
      TOKEN_ENV,
      "-e",
      `CARGO_REGISTRIES_CRANE_CREDENTIAL_PROVIDER=cargo:token`,
      "-e",
      "CARGO_TERM_COLOR=never",
      image,
      "sleep",
      "infinity",
    ],
    { env: { [TOKEN_ENV]: stand.token } },
  ).trim();
  const stop = (): void => {
    run("docker", ["rm", "-f", container]);
  };
  try {
    waitReady(stand, container);
    const version = parseCargoVersion(docker(["exec", container, "cargo", "--version"]));
    return { container, image, version, stop };
  } catch (error) {
    stop();
    throw error;
  }
}

interface CargoOptions {
  dir: string;
  home: string;
  token?: string;
  /** Подмена индекса: нужна только мутационной проверке шага «свежий кэш». */
  index?: string;
}

/** Сценарий публикация, индекс, сборка, yank, разрешение, unyank. Возвращает ожидаемые события аудита. */
export function scenario(stand: Stand, client: Client, minor: string): Expected[] {
  const suffix = `${stand.id}_${minor}`;
  const a = `dfa_${suffix}`;
  const b = `dfb_${suffix}`;
  const agent = `cargo/${client.version}`;
  let homes = 0;
  let step = "";

  const freshHome = (): string => {
    homes += 1;
    return `/tmp/h-${minor}-${homes}`;
  };
  const exec = (args: string[], options: CargoOptions): Result =>
    run(
      "docker",
      [
        "exec",
        "-w",
        `/work/${options.dir}`,
        "-e",
        `CARGO_HOME=${options.home}`,
        "-e",
        `CARGO_TARGET_DIR=${options.home}-target`,
        "-e",
        TOKEN_ENV,
        ...(options.index === undefined
          ? []
          : ["-e", `CARGO_REGISTRIES_CRANE_INDEX=sparse+${options.index}`]),
        client.container,
        "cargo",
        ...args,
      ],
      { env: { [TOKEN_ENV]: options.token ?? stand.token } },
    );
  const must = (args: string[], options: CargoOptions): Result => {
    const result = exec(args, options);
    if (result.status !== 0) {
      throw new Error(
        `${step}: cargo ${args.join(" ")} -> ${result.status}\n${tail(result.stderr)}`,
      );
    }
    return result;
  };
  const expectOutput = (result: Result, want: string): void => {
    if (result.stdout.trim() !== want) {
      throw new Error(
        `${step}: ожидалось «${want}», получено «${result.stdout.trim()}»\n${tail(result.stderr)}`,
      );
    }
  };
  const put = (path: string, content: string): void => {
    runOk("docker", ["exec", "-i", client.container, "tee", `/work/${path}`], { input: content });
  };
  const manifest = (name: string, version: string, deps: string): string =>
    [
      "[package]",
      `name = "${name}"`,
      `version = "${version}"`,
      'edition = "2021"',
      'description = "CRANE dogfood"',
      'license = "MIT"',
      "",
      "[dependencies]",
      deps,
      "",
    ].join("\n");
  const dep = (name: string, req: string): string =>
    `${name} = { version = "${req}", registry = "${REGISTRY}" }`;
  const newCrate = (name: string, kind: "--lib" | "--bin"): void => {
    runOk("docker", [
      "exec",
      "-w",
      "/work",
      client.container,
      "cargo",
      "new",
      "--vcs",
      "none",
      kind,
      name,
    ]);
  };
  const lib = (name: string, version: string, body: string, deps = ""): void => {
    put(`${name}/Cargo.toml`, manifest(name, version, deps));
    put(`${name}/src/lib.rs`, `${body}\n`);
  };
  const app = (name: string, dependency: string, call: string): void => {
    newCrate(name, "--bin");
    put(`${name}/Cargo.toml`, manifest(name, "0.1.0", dependency));
    put(`${name}/src/main.rs`, `fn main() {\n    println!("{}", ${call});\n}\n`);
  };

  const main = "/tmp/h-main";
  const events: Expected[] = [];
  const api = (operation: "publish" | "yank" | "unyank", name: string, version: string): void => {
    events.push({ kind: "api", agent, operation, purl: `pkg:cargo/${name}@${version}` });
  };
  const versionFn = (version: string): string =>
    `pub fn version() -> &'static str {\n    "${version}"\n}`;

  step = "1 публикация a@0.1.0, a@0.1.1, b@0.1.0";
  newCrate(a, "--lib");
  lib(a, "0.1.0", versionFn("0.1.0"));
  must(["publish", "--registry", REGISTRY], { dir: a, home: main });
  api("publish", a, "0.1.0");
  lib(a, "0.1.1", versionFn("0.1.1"));
  must(["publish", "--registry", REGISTRY], { dir: a, home: main });
  api("publish", a, "0.1.1");
  newCrate(b, "--lib");
  lib(b, "0.1.0", `pub fn a_version() -> &'static str {\n    ${a}::version()\n}`, dep(a, "0.1"));
  must(["publish", "--registry", REGISTRY], { dir: b, home: main });
  api("publish", b, "0.1.0");

  step = "2 поиск в индексе";
  const info = must(["info", a, "--registry", REGISTRY], { dir: ".", home: main });
  if (!`${info.stdout}${info.stderr}`.includes("0.1.1")) {
    throw new Error(`${step}: cargo info не показал 0.1.1\n${tail(info.stdout)}`);
  }

  step = "3 сборка app из b (транзитивно a@0.1.1)";
  app("app", dep(b, "0.1"), `${b}::a_version()`);
  expectOutput(must(["run", "-q"], { dir: "app", home: main }), "0.1.1");

  step = "4 yank чужим токеном отклонён";
  const denied = exec(["yank", "--registry", REGISTRY, "--version", "0.1.0", a], {
    dir: a,
    home: main,
    token: stand.wrongToken,
  });
  if (denied.status === 0) {
    throw new Error(`${step}: yank с неверным токеном прошёл`);
  }

  step = "5 yank a@0.1.1";
  must(["yank", "--registry", REGISTRY, "--version", "0.1.1", a], { dir: a, home: main });
  api("yank", a, "0.1.1");

  step = "6 разрешение обходит yanked: app2 получает a@0.1.0";
  app("app2", dep(b, "0.1"), `${b}::a_version()`);
  expectOutput(must(["run", "-q"], { dir: "app2", home: freshHome() }), "0.1.0");

  step = "7 прямая зависимость от yanked версии не разрешается";
  app("app3", dep(a, "=0.1.1"), `${a}::version()`);
  const locked = exec(["generate-lockfile"], { dir: "app3", home: freshHome() });
  if (locked.status === 0 || !YANKED.test(locked.stderr)) {
    throw new Error(`${step}: generate-lockfile -> ${locked.status}\n${tail(locked.stderr)}`);
  }

  step = "8 Cargo.lock с yanked версией собирается из пустого кэша";
  expectOutput(must(["run", "-q", "--locked"], { dir: "app", home: freshHome() }), "0.1.1");

  step = "9 unyank a@0.1.1";
  must(["yank", "--registry", REGISTRY, "--undo", "--version", "0.1.1", a], {
    dir: a,
    home: main,
  });
  api("unyank", a, "0.1.1");
  app("app4", dep(b, "0.1"), `${b}::a_version()`);
  expectOutput(must(["run", "-q"], { dir: "app4", home: freshHome() }), "0.1.1");

  events.push({ kind: "authFailure", agent, min: 1 });
  return events;
}
