// Полный dogfood: CRANE публикует сам себя в hosted-реестр `internal` и собирается из него: внутренние
// крейты из hosted, внешние через proxy crates.io. Заодно два доказательства о proxy: схлопывание
// запросов холодного ключа и сборка при недоступном upstream. Офлайн необратим, поэтому сценарий
// идёт в стенде последним.
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";
import { parse } from "smol-toml";
import {
  accessLines,
  cacheDistribution,
  checkCollapse,
  checkOffline,
  checkUpstreamDown,
  type Expected,
  egressLines,
} from "./audit.ts";
import { type CargoClient, REGISTRY, ROOT_CERT, startCargo } from "./cargo.ts";
import { docker, log, MS_PER_SECOND, SECONDS_PER_MINUTE, tail, timed } from "./exec.ts";
import { field, isTable, publishable, type Table } from "./overlay.ts";
import { proxyCredentials, sourceConfig } from "./proxy.ts";
import type { Stand } from "./stand.ts";

/** Тот же набор, что `COPY` в Dockerfile: образ стенда и публикуемые крейты собираются из одних исходников. */
const SOURCES = ["Cargo.toml", "Cargo.lock", "crates"];
// Артефакты сборки внутри crates/ (в Dockerfile их отсекает .dockerignore) и мусор ОС.
const SKIPPED = new Set(["target", ".DS_Store"]);

const WORK = "/work";
const SOURCE_DIR = `${WORK}/crane`;
// Каталоги этого прогона внутри контейнера; постоянного тома нет: иначе кэш прошлого прогона даст ложный зелёный.
const INSTALL_DIR = "install-home";
const PUBLISH_DIR = "publish-home";
const INSTALL_HOME = `${WORK}/${INSTALL_DIR}`;
const PUBLISH_HOME = `${WORK}/${PUBLISH_DIR}`;
const TARGET_DIR = `${WORK}/install-target`;
const INSTALL_ROOT = `${WORK}/install-root`;
const SERVER_CRATE = "crane-server";
const BINARY = "crane";
const USAGE_EXIT = 2;
const USAGE = "использование: crane";

const PUBLISH_TIMEOUT_MINUTES = 30;
const INSTALL_TIMEOUT_MINUTES = 60;
const minutes = (value: number): number => value * SECONDS_PER_MINUTE * MS_PER_SECOND;

// Крейт вне графа CRANE и вне e2e-сценариев: его ключ индекса заведомо холодный.
const COLD_CRATE = "ferris-says";
const COLD_PATH = "fe/rr/ferris-says";
const PARALLEL = 8;
// Проба недоступности: тоже вне графа CRANE, e2e и схлопывания, поэтому в кэше её ключа нет.
const PROBE_PATH = "co/ws/cowsay";

// Закреплён тегом и digest (v0.2.0 п.7): нужен только `ip`.
const BUSYBOX =
  "busybox:1.37.0@sha256:bdf57e528e45e4433820e045b29b4597825a1c9e38353532d90a01445013f82e";
// Маршрутов по умолчанию может быть несколько (по одному на сеть): снимаются все, остаток печатается как диагностика.
const DROP_ROUTES = [
  "while ip route del default 2>/dev/null; do :; done",
  "while ip -6 route del default 2>/dev/null; do :; done",
  // busybox `ip route show default` печатает все маршруты, поэтому ищем строку default сами.
  "! ip route show | grep -q '^default' || { ip route show >&2; exit 1; }",
].join("; ");

interface Workspace {
  version: string;
  /** Пути манифестов относительно корня воркспейса, корневой первым. */
  manifests: string[];
  names: Set<string>;
}

const table = (value: unknown, what: string): Table => {
  if (!isTable(value)) {
    throw new Error(`Cargo.toml: нет таблицы ${what}`);
  }
  return value;
};
const text = (value: unknown, what: string): string => {
  if (typeof value !== "string") {
    throw new Error(`Cargo.toml: нет строки ${what}`);
  }
  return value;
};

/** Состав воркспейса по корневому манифесту: версия, члены и имена их пакетов. */
export function readWorkspace(root: string): Workspace {
  const read = (path: string): Table => table(parse(readFileSync(join(root, path), "utf8")), path);
  const workspace = table(field(read("Cargo.toml"), "workspace"), "workspace");
  const shared = table(field(workspace, "package"), "workspace.package");
  const version = text(field(shared, "version"), "workspace.package.version");
  const members = field(workspace, "members");
  if (!Array.isArray(members) || members.some((m) => typeof m !== "string" || m.includes("*"))) {
    throw new Error("Cargo.toml: members должны быть явным списком путей");
  }
  const manifests = members.map((member) => `${String(member)}/Cargo.toml`);
  const names = new Set(
    manifests.map((path) =>
      text(field(table(field(read(path), "package"), `${path}: package`), "name"), `${path}: name`),
    ),
  );
  return { version, manifests: ["Cargo.toml", ...manifests], names };
}

/** Копия исходников из рабочего дерева (не из git) в каталог `to`, манифесты переписаны под публикацию. */
function stageSources(root: string, to: string, workspace: Workspace): void {
  for (const name of SOURCES) {
    cpSync(join(root, name), join(to, name), {
      recursive: true,
      filter: (path) => !SKIPPED.has(basename(path)),
    });
  }
  for (const path of workspace.manifests) {
    const file = join(to, path);
    writeFileSync(
      file,
      publishable(readFileSync(file, "utf8"), workspace.names, workspace.version, REGISTRY),
    );
  }
}

/** Секреты идут значениями окружения: в argv попадают только имена. */
const credentials = (stand: Stand): Record<string, string> => ({
  CARGO_REGISTRIES_CRANE_TOKEN: stand.token,
  ...proxyCredentials(stand.token),
});

export interface FullOptions {
  /** Корень репозитория на хосте. */
  root: string;
  /** Образ клиента latest. */
  image: string;
  /** Proxy-репозиторий crates.io, отведённый полному сценарию. */
  repo: string;
}

/** Выполняет полный сценарий и возвращает события аудита, которые он обязан породить. */
export function fullScenario(stand: Stand, options: FullOptions): Expected[] {
  const client = startCargo(stand, options.image);
  try {
    return run(stand, client, options);
  } finally {
    client.stop();
  }
}

function run(stand: Stand, client: CargoClient, { root, repo }: FullOptions): Expected[] {
  const base = `/cargo/${repo}`;
  const proxyUrl = `https://localhost${base}`;
  const workspace = readWorkspace(root);
  const config = sourceConfig(proxyUrl);
  const env = (home: string): Record<string, string> => ({
    ...credentials(stand),
    CARGO_HOME: home,
    CARGO_TARGET_DIR: TARGET_DIR,
    // Холодная сборка в debug-профиле без отладочной информации и инкрементального кэша заметно быстрее.
    CARGO_PROFILE_DEV_DEBUG: "0",
    CARGO_INCREMENTAL: "0",
  });
  const must = (cmd: string[], options: { cwd?: string; home: string; timeoutMs: number }) => {
    const result = client.exec(cmd, {
      ...(options.cwd === undefined ? {} : { cwd: options.cwd }),
      env: env(options.home),
      timeoutMs: options.timeoutMs,
    });
    if (result.status !== 0) {
      throw new Error(`${cmd.join(" ")} -> ${result.status}\n${tail(result.stderr)}`);
    }
    return result;
  };
  const install = (extra: string[]): void => {
    must(
      [
        "cargo",
        "install",
        SERVER_CRATE,
        "--registry",
        REGISTRY,
        "--version",
        workspace.version,
        "--locked",
        "--debug",
        "--root",
        INSTALL_ROOT,
        ...extra,
      ],
      { home: INSTALL_HOME, timeoutMs: minutes(INSTALL_TIMEOUT_MINUTES) },
    );
  };
  const proxyLines = (from = 0) =>
    accessLines(stand.craneLogs())
      .slice(from)
      .filter((line) => line.path.startsWith(`${base}/`));

  timed("копия исходников и overlay", () => {
    const staged = mkdtempSync(join(tmpdir(), "crane-src-"));
    try {
      stageSources(root, staged, workspace);
      client.copyIn(staged, SOURCE_DIR);
    } finally {
      rmSync(staged, { recursive: true, force: true });
    }
    client.write(`${PUBLISH_DIR}/config.toml`, config);
    client.write(`${INSTALL_DIR}/config.toml`, config);
  });

  timed(`cargo publish --workspace (${workspace.names.size} крейтов)`, () => {
    // --no-verify: проверочная сборка каждого крейта удвоила бы время, а дефекты упаковки крейтов вне
    // замыкания сервера иначе всплывут только у первого потребителя. Замыкание сервера проверяет install ниже.
    must(["cargo", "publish", "--workspace", "--registry", REGISTRY, "--no-verify"], {
      cwd: SOURCE_DIR,
      home: PUBLISH_HOME,
      timeoutMs: minutes(PUBLISH_TIMEOUT_MINUTES),
    });
  });

  timed(`схлопывание: ${PARALLEL} параллельных запросов холодного ключа ${COLD_CRATE}`, () => {
    const url = `${proxyUrl}/index/${COLD_PATH}`;
    const fetch = client.exec(
      [
        "curl",
        "-sS",
        "--parallel",
        "--parallel-immediate",
        "--parallel-max",
        String(PARALLEL),
        "--cacert",
        ROOT_CERT,
        "-K",
        "-",
        ...Array.from({ length: PARALLEL }, () => ["-o", "/dev/null", url]).flat(),
      ],
      { input: `header = "Authorization: ${stand.token}"\n` },
    );
    if (fetch.status !== 0) {
      throw new Error(`curl -> ${fetch.status}\n${tail(fetch.stderr)}`);
    }
    const logs = stand.craneLogs();
    const problems = checkCollapse(accessLines(logs), egressLines(logs), {
      accessPath: `${base}/index/${COLD_PATH}`,
      upstreamPath: COLD_PATH,
      requests: PARALLEL,
    });
    if (problems.length > 0) {
      throw new Error(`схлопывание: ${problems.join("; ")}`);
    }
  });

  timed("cargo install crane-server из hosted и proxy (пустой CARGO_HOME)", () => {
    install([]);
    const started = client.exec([`${INSTALL_ROOT}/bin/${BINARY}`]);
    // У бинарника нет --version: без роли он печатает использование и завершается кодом 2.
    if (started.status !== USAGE_EXIT || !`${started.stdout}${started.stderr}`.includes(USAGE)) {
      throw new Error(`${BINARY} без аргументов -> ${started.status}\n${tail(started.stdout)}`);
    }
  });

  timed("офлайн: маршрут по умолчанию удалён, повторный install", () => {
    // Шаг устроен так, чтобы при Valkey L2 между прогревом и отключением добавлялся рестарт crane (одна строка:
    // docker restart stand.crane) и проверялось, что кэш переживает процесс. Сейчас кэш в памяти: не рестартовать.
    docker([
      "run",
      "--rm",
      "--network",
      `container:${stand.crane}`,
      "--cap-add",
      "NET_ADMIN",
      BUSYBOX,
      "sh",
      "-c",
      DROP_ROUTES,
    ]);
    // Обе отметки берутся вместе: egress-строки до них относятся к прогреву.
    const marked = stand.craneLogs();
    const mark = { access: accessLines(marked).length, egress: egressLines(marked).length };
    must(["rm", "-rf", `${INSTALL_HOME}/registry`], { home: INSTALL_HOME, timeoutMs: minutes(1) });
    // Тот же CARGO_TARGET_DIR: перекомпиляции быть не должно; --force: без него cargo молча ничего не делает.
    install(["--force"]);
    const lines = proxyLines(mark.access);
    log(
      `офлайн: ответов proxy ${lines.length}, cache: ${JSON.stringify(cacheDistribution(lines))}`,
    );
    const problems = checkOffline(lines);
    if (problems.length > 0) {
      throw new Error(`офлайн: ${problems.join("; ")}`);
    }
    // Попадания в кэш в upstream не ходят: недоступность доказывает только проба ключа, которого в кэше нет.
    const probe = client.exec(
      [
        "curl",
        "-sS",
        "--cacert",
        ROOT_CERT,
        "-K",
        "-",
        "-o",
        "/dev/null",
        `${proxyUrl}/index/${PROBE_PATH}`,
      ],
      { input: `header = "Authorization: ${stand.token}"\n` },
    );
    if (probe.status !== 0) {
      throw new Error(`curl -> ${probe.status}\n${tail(probe.stderr)}`);
    }
    const after = stand.craneLogs();
    const down = checkUpstreamDown(
      accessLines(after)
        .slice(mark.access)
        .filter((line) => line.path === `${base}/index/${PROBE_PATH}`),
      egressLines(after).slice(mark.egress),
    );
    if (down.length > 0) {
      throw new Error(`upstream недоступен: ${down.join("; ")}`);
    }
  });

  return [...workspace.names].map((name) => ({
    kind: "api",
    agent: `cargo/${client.version}`,
    operation: "publish",
    purl: `pkg:cargo/${name}@${workspace.version}`,
  }));
}
