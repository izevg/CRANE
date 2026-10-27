import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { checkAaaTree } from "./aaa.ts";
import { cargoMetadata, checkRustLayers, checkStructure, type Rules } from "./checks.ts";

const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../..");
const rules = JSON.parse(readFileSync(resolve(here, "rules.json"), "utf8")) as Rules;

const violations = [
  ...checkStructure(root, rules),
  ...checkRustLayers(root, rules, cargoMetadata(root)),
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
