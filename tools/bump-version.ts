// Использование: node tools/bump-version.ts v0.1.0
// Проставляет версию в Cargo.toml (workspace), package.json и Chart.yaml (если есть).
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const WORKSPACE_VERSION = /(\[workspace\.package\][^[]*?\nversion = ")[^"]+(")/s;
const JSON_VERSION = /("version":\s*")[^"]+(")/;
const CHART_VERSION = /^version:.*$/m;
const CHART_APP_VERSION = /^appVersion:.*$/m;

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const tag = process.argv[2] ?? "";
const match = /^v(\d+\.\d+\.\d+)$/.exec(tag);
if (!match) {
  process.stderr.write(`ожидается тег вида v1.2.3, получено «${tag}»\n`);
  process.exit(1);
}
const version = match[1] ?? "";

const edit = (path: string, fn: (text: string) => string): void => {
  const file = join(root, path);
  if (!existsSync(file)) {
    return;
  }
  const before = readFileSync(file, "utf8");
  const after = fn(before);
  if (before === after && !before.includes(version)) {
    throw new Error(`${path}: версия не найдена`);
  }
  writeFileSync(file, after);
  process.stdout.write(`${path} → ${version}\n`);
};

edit("Cargo.toml", (s) => s.replace(WORKSPACE_VERSION, `$1${version}$2`));
for (const pkg of ["package.json", "web/package.json"]) {
  edit(pkg, (s) => s.replace(JSON_VERSION, `$1${version}$2`));
}
const helm = join(root, "deploy/helm");
if (existsSync(helm)) {
  for (const chart of readdirSync(helm)) {
    edit(`deploy/helm/${chart}/Chart.yaml`, (s) =>
      s
        .replace(CHART_VERSION, `version: ${version}`)
        .replace(CHART_APP_VERSION, `appVersion: "${version}"`),
    );
  }
}
