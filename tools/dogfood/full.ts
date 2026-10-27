// Полный dogfood: CRANE публикует сам себя в hosted-реестр `internal` и собирается из него: внутренние
// крейты из hosted, внешние через proxy crates.io. Заодно два доказательства о proxy: схлопывание
// запросов холодного ключа и сборка при недоступном upstream. Офлайн необратим, поэтому сценарий
// идёт в стенде последним.
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { basename, join, relative } from "node:path";
import { cargoMetadata } from "../lint-structure/checks.ts";
import {
  accessLines,
  cacheDistribution,
  checkCollapse,
  checkOffline,
  checkUpstreamDown,
  type Expected,
  egressLines,
} from "./audit.ts";
import {
  authorizedCurl,
  type CargoClient,
  craneCredentials,
  REGISTRY,
  startCargo,
} from "./cargo.ts";
import { docker, log, MS_PER_SECOND, SECONDS_PER_MINUTE, tail, timed } from "./exec.ts";
import { publishable } from "./overlay.ts";
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

/** Состав воркспейса из `cargo metadata`: имена членов, их манифесты и общая версия. */
function readWorkspace(root: string): Workspace {
  const { packages } = cargoMetadata(root);
  const versions = new Set(packages.map((pkg) => pkg.version));
  const [version] = versions;
  if (versions.size !== 1 || version === undefined) {
    throw new Error(
      `члены воркспейса должны делить одну версию, найдено: ${[...versions].join(", ")}`,
    );
  }
  return {
    version,
    manifests: ["Cargo.toml", ...packages.map((pkg) => relative(root, pkg.manifest_path))],
    names: new Set(packages.map((pkg) => pkg.name)),
  };
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

export interface FullOptions {
  /** Корень репозитория на хосте. */
  root: string;
  /** Образ клиента latest. */
  image: string;
  /** Proxy-репозиторий crates.io, отведённый полному сценарию. */
  repo: string;
}

interface Context {
  stand: Stand;
  client: CargoClient;
  root: string;
  workspace: Workspace;
  /** Путь proxy-репозитория на crane: `/cargo/<repo>`. */
  base: string;
  /** Тот же репозиторий как URL для клиента. */
  proxyUrl: string;
}

/** Выполняет полный сценарий и возвращает события аудита, которые он обязан породить. */
export function fullScenario(stand: Stand, options: FullOptions): Expected[] {
  const client = startCargo(stand, options.image);
  try {
    const base = `/cargo/${options.repo}`;
    const context: Context = {
      stand,
      client,
      root: options.root,
      workspace: readWorkspace(options.root),
      base,
      proxyUrl: `https://localhost${base}`,
    };
    const { workspace } = context;
    timed("копия исходников и overlay", () => stage(context));
    timed(`cargo publish --workspace (${workspace.names.size} крейтов)`, () => publish(context));
    timed(`схлопывание: ${PARALLEL} параллельных запросов холодного ключа`, () =>
      collapse(context),
    );
    timed("cargo install crane-server из hosted и proxy (пустой CARGO_HOME)", () =>
      installFromRegistry(context),
    );
    timed("офлайн: маршрут по умолчанию удалён, повторный install", () => offline(context));
    return [...workspace.names].map((name) => ({
      kind: "api",
      agent: `cargo/${client.version}`,
      operation: "publish",
      purl: `pkg:cargo/${name}@${workspace.version}`,
    }));
  } finally {
    client.stop();
  }
}

/** Окружение cargo в контейнере: токены обоих реестров, фиксированные каталоги этого прогона. */
const cargoEnv = (stand: Stand, home: string): Record<string, string> => ({
  ...craneCredentials(stand.token),
  ...proxyCredentials(stand.token),
  CARGO_HOME: home,
  CARGO_TARGET_DIR: TARGET_DIR,
  // Холодная сборка в debug-профиле без отладочной информации и инкрементального кэша заметно быстрее.
  CARGO_PROFILE_DEV_DEBUG: "0",
  CARGO_INCREMENTAL: "0",
});

function must(
  { client, stand }: Context,
  cmd: string[],
  options: { cwd?: string; home: string; timeoutMs: number },
): void {
  const result = client.exec(cmd, {
    ...(options.cwd === undefined ? {} : { cwd: options.cwd }),
    env: cargoEnv(stand, options.home),
    timeoutMs: options.timeoutMs,
  });
  if (result.status !== 0) {
    throw new Error(`${cmd.join(" ")} -> ${result.status}\n${tail(result.stderr)}`);
  }
}

function install(context: Context, extra: string[]): void {
  must(
    context,
    [
      "cargo",
      "install",
      SERVER_CRATE,
      "--registry",
      REGISTRY,
      "--version",
      context.workspace.version,
      "--locked",
      "--debug",
      "--root",
      INSTALL_ROOT,
      ...extra,
    ],
    { home: INSTALL_HOME, timeoutMs: minutes(INSTALL_TIMEOUT_MINUTES) },
  );
}

/** Строки access этого proxy-репозитория, начиная с позиции `from` общего списка. */
const proxyLines = ({ stand, base }: Context, from = 0) =>
  accessLines(stand.craneLogs())
    .slice(from)
    .filter((line) => line.path.startsWith(`${base}/`));

function stage({ client, root, workspace, proxyUrl }: Context): void {
  const staged = mkdtempSync(join(tmpdir(), "crane-src-"));
  try {
    stageSources(root, staged, workspace);
    client.copyIn(staged, SOURCE_DIR);
  } finally {
    rmSync(staged, { recursive: true, force: true });
  }
  const config = sourceConfig(proxyUrl);
  client.write(`${PUBLISH_DIR}/config.toml`, config);
  client.write(`${INSTALL_DIR}/config.toml`, config);
}

function publish(context: Context): void {
  // --no-verify: проверочная сборка каждого крейта удвоила бы время, а дефекты упаковки крейтов вне
  // замыкания сервера иначе всплывут только у первого потребителя. Замыкание сервера проверяет install.
  must(context, ["cargo", "publish", "--workspace", "--registry", REGISTRY, "--no-verify"], {
    cwd: SOURCE_DIR,
    home: PUBLISH_HOME,
    timeoutMs: minutes(PUBLISH_TIMEOUT_MINUTES),
  });
}

/** Параллельные запросы холодного ключа: все miss и ровно одна выборка из upstream. */
function collapse({ client, stand, base, proxyUrl }: Context): void {
  const url = `${proxyUrl}/index/${COLD_PATH}`;
  const fetch = authorizedCurl(client, stand.token, [
    "-sS",
    "--parallel",
    "--parallel-immediate",
    "--parallel-max",
    String(PARALLEL),
    ...Array.from({ length: PARALLEL }, () => ["-o", "/dev/null", url]).flat(),
  ]);
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
}

function installFromRegistry(context: Context): void {
  install(context, []);
  const started = context.client.exec([`${INSTALL_ROOT}/bin/${BINARY}`]);
  // У бинарника нет --version: без роли он печатает использование и завершается кодом 2.
  if (started.status !== USAGE_EXIT || !`${started.stdout}${started.stderr}`.includes(USAGE)) {
    throw new Error(`${BINARY} без аргументов -> ${started.status}\n${tail(started.stdout)}`);
  }
}

/** Снимает все маршруты по умолчанию в сетевом пространстве crane: DNS отвечает, connect отказывает сразу. */
function cutUpstream(stand: Stand): void {
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
}

function offline(context: Context): void {
  const { client, stand, base, proxyUrl } = context;
  // Стенд идёт с CRANE_CACHE=memory из .env.dist, поэтому crane не перезапускается. Переход стенда на профиль
  // cache (Valkey L2) с рестартом crane между прогревом и отключением — отдельное решение.
  cutUpstream(stand);
  // Обе отметки берутся вместе: egress-строки до них относятся к прогреву.
  const marked = stand.craneLogs();
  const mark = { access: accessLines(marked).length, egress: egressLines(marked).length };
  must(context, ["rm", "-rf", `${INSTALL_HOME}/registry`], {
    home: INSTALL_HOME,
    timeoutMs: minutes(1),
  });
  // Тот же CARGO_TARGET_DIR: перекомпиляции быть не должно; --force: без него cargo молча ничего не делает.
  install(context, ["--force"]);
  const lines = proxyLines(context, mark.access);
  log(`офлайн: ответов proxy ${lines.length}, cache: ${JSON.stringify(cacheDistribution(lines))}`);
  const problems = checkOffline(lines);
  if (problems.length > 0) {
    throw new Error(`офлайн: ${problems.join("; ")}`);
  }
  // Попадания в кэш в upstream не ходят: недоступность доказывает только проба ключа, которого в кэше нет.
  const probe = authorizedCurl(client, stand.token, [
    "-sS",
    "-o",
    "/dev/null",
    `${proxyUrl}/index/${PROBE_PATH}`,
  ]);
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
}
