// Dogfood и e2e: одноразовый стенд compose и настоящий cargo.
// Использование: node tools/dogfood/index.ts [--cargo latest|latest,previous]
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { type Expected, leaks, verify } from "./audit.ts";
import { scenario, startCargo } from "./cargo.ts";
import { docker, log, MS_PER_SECOND, warn } from "./exec.ts";
import { proxyScenario } from "./proxy.ts";
import { describeStand, type Stand } from "./stand.ts";
import { parseCargo } from "./versions.ts";

const ROOT = join(fileURLToPath(import.meta.url), "..", "..", "..");
// Клиент делит сетевое пространство имён с Caddy и ходит на localhost: сервер увидит loopback.
const LOOPBACK = ["127.0.0.1", "::1"];

type Which = "latest" | "previous";

interface Target {
  image: string;
  minor: number;
}

function parseArgs(argv: string[]): Which[] {
  const flag = argv.indexOf("--cargo");
  if (flag === -1) {
    return ["latest"];
  }
  const values = (argv[flag + 1] ?? "").split(",");
  const known = values.filter((v): v is Which => v === "latest" || v === "previous");
  if (known.length === 0 || known.length !== values.length) {
    throw new Error("--cargo принимает latest, previous или их список через запятую");
  }
  return [...new Set(known)];
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
    return { image, minor };
  });
}

/** Сценарий на каждой версии cargo, затем сверка аудита и поиск утечек. Возвращает число проблем. */
function exercise(stand: Stand, targets: Target[]): number {
  const expected: Expected[] = [];
  const versions: string[] = [];
  for (const [index, target] of targets.entries()) {
    const client = startCargo(stand, target.image);
    try {
      log(`${target.image}: cargo ${client.version}`);
      versions.push(client.version);
      expected.push(...scenario(stand, client, target.minor));
      log(`${target.image}: сценарий пройден`);
      proxyScenario(stand, client, target.minor, index);
      log(`${target.image}: proxy crates.io пройден`);
    } finally {
      client.stop();
    }
  }
  const mismatches = verify(stand.craneLogs(), expected, {
    principal: stand.principal,
    loopback: LOOPBACK,
  });
  const leaked = leaks(stand.allLogs(), stand.secrets);
  for (const m of mismatches) {
    warn(`аудит: [${m.check}] ${m.detail}`);
  }
  for (const name of leaked) {
    warn(`утечка: ${name} найден в журналах контейнеров`);
  }
  log(
    `cargo: ${versions.join(", ")}; ожидаемых событий аудита: ${expected.length}; ` +
      `расхождений: ${mismatches.length}; утечек: ${leaked.length}`,
  );
  return mismatches.length + leaked.length;
}

const logPath = (stand: Stand): string => join(ROOT, "out", "dogfood", `${stand.project}.log`);

function main(): number {
  const started = Date.now();
  const targets = resolveTargets(parseArgs(process.argv.slice(2)));
  const stand = describeStand(ROOT);
  let problems = 1;
  let leftovers: string[] = [];
  try {
    stand.start();
    problems = exercise(stand, targets);
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
