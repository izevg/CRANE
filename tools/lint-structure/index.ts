import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { checkAaaTree } from "./aaa.ts";
import { cargoMetadata, checkRustLayers, checkStructure, type Rules } from "./checks.ts";
import { measure, oversized, sizeReport } from "./size.ts";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");
const rules = JSON.parse(readFileSync(resolve(here, "rules.json"), "utf8")) as Rules;

// Сколько самых больших файлов показывает отчёт, даже если они ниже порога предупреждения.
const REPORT_TOP = 20;

if (process.argv.includes("--report")) {
  const { warnLines, maxLines } = rules.size;
  const report = sizeReport(measure(root, rules), REPORT_TOP, warnLines, maxLines);
  process.stdout.write(`${report.join("\n")}\n`);
  process.exit(0);
}

const violations = [
  ...checkStructure(root, rules),
  ...checkRustLayers(root, rules, cargoMetadata(root)),
  ...oversized(measure(root, rules), rules.size.maxLines),
  ...checkAaaTree(root, rules),
];

if (violations.length > 0) {
  process.stderr.write(`lint-structure: ${violations.length} нарушений\n`);
  for (const v of violations) {
    process.stderr.write(`  - ${v}\n`);
  }
  process.exit(1);
}
process.stdout.write("lint-structure: ок\n");
