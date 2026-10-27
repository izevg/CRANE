// Dogfood и e2e: одноразовый стенд compose и настоящий cargo.
// Использование: node tools/dogfood/index.ts [--cargo latest|latest,previous] [--full]
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { type Expected, leaks, verify } from "./audit.ts";
import { hostedName, scenario, startCargo } from "./cargo.ts";
import { docker, log, MS_PER_SECOND, timed, warn } from "./exec.ts";
import { fullScenario } from "./full.ts";
import { npmScenario } from "./npm.scenario.ts";
import { startNpm } from "./npm.ts";
import { proxyScenario } from "./proxy.ts";
import { describeStand, NAMESPACES, proxyRepo, type Stand } from "./stand.ts";
import { parseArgs, parseCargo, type Which } from "./versions.ts";

const ROOT = join(fileURLToPath(import.meta.url), "..", "..", "..");
// Клиент делит сетевое пространство имён с Caddy и ходит на localhost: сервер увидит loopback.
const LOOPBACK = ["127.0.0.1", "::1"];

interface Target {
  which: Which;
  image: string;
  minor: number;
}

/** Образы rust:1.M-bookworm и rust:1.(M-1)-bookworm по самой свежей стабильной версии. */
function resolveTargets(which: Which[]): Target[] {
  docker(["pull", "-q", "rust:1-bookworm"]);
  const latest = parseCargo(docker(["run", "--rm", "rust:1-bookworm", "cargo", "--version"])).minor;
  const minors: Record<Which, number> = { latest, previous: latest - 1 };
  return which.map((w) => {
    const minor = minors[w];
    const image = `rust:1.${minor}-bookworm`;
    docker(["pull", "-q", image]);
    return { which: w, image, minor };
  });
}

/** Сценарий npm на последней версии клиента: после целей cargo и до полного сценария. */
function exerciseNpm(stand: Stand): { version: string; events: Expected[] } {
  const client = startNpm(stand);
  try {
    log(`npm ${client.version}: клиент готов`);
    const events = npmScenario(stand, client);
    log(`npm ${client.version}: сценарий пройден`);
    return { version: client.version, events };
  } finally {
    client.stop();
  }
}

/** Сценарий на каждой версии cargo, затем сверка аудита и поиск утечек. Возвращает число проблем. */
function exercise(stand: Stand, targets: Target[], root: string, full: boolean): number {
  const expected: Expected[] = [];
  const versions: string[] = [];
  for (const [index, target] of targets.entries()) {
    const client = startCargo(stand, target.image);
    try {
      log(`${target.image}: cargo ${client.version}`);
      versions.push(client.version);
      expected.push(...scenario(stand, client, target.minor));
      log(`${target.image}: сценарий пройден`);
      expected.push(
        ...proxyScenario(
          stand,
          client,
          target.minor,
          proxyRepo(index),
          hostedName(stand, target.minor),
        ),
      );
      log(`${target.image}: proxy crates.io пройден`);
    } finally {
      client.stop();
    }
  }
  const npm = exerciseNpm(stand);
  expected.push(...npm.events);
  const latest = targets.find((target) => target.which === "latest");
  if (full && latest !== undefined) {
    // Последним: офлайн необратим, после него стенд пригоден только для сверки журналов.
    expected.push(
      ...timed("полный сценарий", () =>
        fullScenario(stand, { root, image: latest.image, repo: proxyRepo(targets.length) }),
      ),
    );
    log("полный сценарий пройден");
  }
  const mismatches = verify(stand.craneLogs(), expected, {
    principal: stand.principal,
    loopback: LOOPBACK,
    configReservations: NAMESPACES.length,
  });
  const leaked = leaks(stand.allLogs(), stand.secrets);
  for (const m of mismatches) {
    warn(`аудит: [${m.check}] ${m.detail}`);
  }
  for (const name of leaked) {
    warn(`утечка: ${name} найден в журналах контейнеров`);
  }
  log(
    `cargo: ${versions.join(", ")}; npm: ${npm.version}; ожидаемых событий аудита: ${expected.length}; ` +
      `расхождений: ${mismatches.length}; утечек: ${leaked.length}`,
  );
  return mismatches.length + leaked.length;
}

const logPath = (stand: Stand): string => join(ROOT, "out", "dogfood", `${stand.project}.log`);

function main(): number {
  const started = Date.now();
  const args = parseArgs(process.argv.slice(2));
  const targets = timed("образы rust", () => resolveTargets(args.which));
  // Полному сценарию отведён свой proxy-репозиторий после репозиториев целей cargo.
  const repos = targets.map((_, index) => proxyRepo(index));
  const stand = describeStand(ROOT, args.full ? [...repos, proxyRepo(repos.length)] : repos);
  let problems = 1;
  let leftovers: string[] = [];
  try {
    timed("стенд: сборка образа и compose up", () => stand.start());
    problems = timed("сценарии", () => exercise(stand, targets, ROOT, args.full));
  } catch (error) {
    warn(error instanceof Error ? error.message : String(error));
  } finally {
    if (problems > 0) {
      saveLogs(stand);
    }
    leftovers = stand.down();
  }
  for (const item of leftovers) {
    warn(`после down осталось: ${item}`);
  }
  const seconds = Math.round((Date.now() - started) / MS_PER_SECOND);
  const ok = problems === 0 && leftovers.length === 0;
  log(`${ok ? "OK" : "FAIL"}: ${seconds} с`);
  return ok ? 0 : 1;
}

function saveLogs(stand: Stand): void {
  try {
    stand.saveLogs(logPath(stand));
    warn(`журналы сохранены: ${logPath(stand)}`);
  } catch {
    // журналов может не быть: стенд мог не успеть подняться
  }
}

// Сигналы не прерывают процесс: очистка стенда выполняется только в finally.
process.on("SIGINT", () => undefined);
process.on("SIGTERM", () => undefined);
process.exitCode = main();
