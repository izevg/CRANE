import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { checkTree } from "./checks.ts";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const violations = checkTree(root);

if (violations.length > 0) {
  process.stderr.write(`lint-i18n: ${violations.length} нарушений\n`);
  for (const v of violations) {
    process.stderr.write(`  - ${v}\n`);
  }
  process.exit(1);
}
process.stdout.write("lint-i18n: ок\n");
