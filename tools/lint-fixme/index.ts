import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import {
  check,
  type Fixme,
  MARKERS_PATTERN,
  parseFixme,
  parseGrep,
  parseMilestones,
  parseOperations,
  report,
  withNext,
} from "./checks.ts";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

const CODE_ROOTS = ["crates", "cli", "web/src", "web/screens", "web/test", "tools"];
const EXTENSIONS = ["rs", "ts", "tsx"];
const GREP_NO_MATCH = 1;
const BYTES_PER_KIB = 1024;
const GREP_MAX_BUFFER_MIB = 64;
const GREP_MAX_BUFFER = GREP_MAX_BUFFER_MIB * BYTES_PER_KIB * BYTES_PER_KIB;
const CONTRACTS = ["docs/api/admin.openapi.yaml", "docs/api/common.yaml"];

// Единственный допустимый маркер долга - FIXME(vX.Y.Z); TODO/XXX/HACK ловятся тем же проходом.
// Сканируем только код: в estimate-agent-hours.py и *.md FIXME упоминается как термин, а не заглушка.
const pathspecs = [
  ...CODE_ROOTS.flatMap((dir) => EXTENSIONS.map((ext) => `:(glob)${dir}/**/*.${ext}`)),
  ":(exclude)tools/lint-fixme/**",
  ":(exclude,glob)**/generated/**",
  ":(exclude,glob)**/migrations/**",
];

const grepFixme = (): string => {
  const result = spawnSync(
    "git",
    ["grep", "-n", "-I", "--no-color", "-w", "-E", "-e", MARKERS_PATTERN, "--", ...pathspecs],
    {
      cwd: root,
      encoding: "utf8",
      maxBuffer: GREP_MAX_BUFFER,
    },
  );
  // git grep отвечает 1, когда совпадений нет: это пустой реестр, а не ошибка.
  if (result.status !== 0 && result.status !== GREP_NO_MATCH) {
    process.stderr.write(
      `lint-fixme: git grep завершился с ошибкой: ${result.error?.message ?? result.stderr}\n`,
    );
    process.exit(2);
  }
  return result.stdout;
};

const read = (path: string): string => readFileSync(resolve(root, path), "utf8");
// Файл с несколькими заглушками читаем один раз.
const sources = new Map<string, string>();
const source = (file: string): string => {
  const cached = sources.get(file) ?? read(file);
  sources.set(file, cached);
  return cached;
};

const fixmes: Fixme[] = [];
const invalid: string[] = [];
for (const hit of parseGrep(grepFixme())) {
  const parsed = parseFixme(hit);
  if (typeof parsed === "string") {
    invalid.push(parsed);
  } else {
    fixmes.push(withNext(parsed, source(parsed.file)));
  }
}

const milestones = parseMilestones(read("ROADMAP.md"));

if (process.argv.includes("--report")) {
  process.stdout.write(`${report(fixmes, invalid, milestones).join("\n")}\n`);
  process.exit(0);
}

const operations = CONTRACTS.flatMap((path) => parseOperations(read(path), path));
const violations = [...invalid, ...check(fixmes, milestones, operations)];

if (violations.length > 0) {
  process.stderr.write(`lint-fixme: ${violations.length} нарушений\n`);
  for (const v of violations) {
    process.stderr.write(`  - ${v}\n`);
  }
  process.exit(1);
}
process.stdout.write("lint-fixme: ок\n");
