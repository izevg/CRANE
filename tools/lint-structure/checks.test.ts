import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test as it } from "node:test";
import { type CargoMetadata, checkRustLayers, checkStructure, type Rules } from "./checks.ts";

const MAX_DIRS = 8;

const rules: Rules = {
  scope: ["src"],
  limits: { files: 4, dirs: MAX_DIRS },
  ignoredDirs: ["node_modules"],
  unlimitedDirs: ["tests"],
  serviceFiles: ["Cargo.toml"],
  testFiles: ["*.test.*"],
  entryFiles: ["lib.rs", "index.ts"],
  rustLayers: { domain: [], ports: ["domain"], adapters: ["domain", "ports"] },
};

const tree = (spec: Record<string, 0 | 1>): string => {
  const root = mkdtempSync(join(tmpdir(), "ls-"));
  for (const [path, isDir] of Object.entries(spec)) {
    if (isDir) {
      mkdirSync(join(root, path), { recursive: true });
    } else {
      mkdirSync(join(root, path, ".."), { recursive: true });
      writeFileSync(join(root, path), "");
    }
  }
  return root;
};

it("4 файла в папке допустимы, 5 — нарушение", () => {
  const ok = tree({ "src/a.ts": 0, "src/b.ts": 0, "src/c.ts": 0, "src/d.ts": 0 });
  assert.deepEqual(checkStructure(ok, rules), []);
  const bad = tree({ "src/a.ts": 0, "src/b.ts": 0, "src/c.ts": 0, "src/d.ts": 0, "src/e.ts": 0 });
  assert.equal(checkStructure(bad, rules).length, 1);
});

it("8 подпапок допустимы, 9 — нарушение", () => {
  const dirs = (n: number): Record<string, 0 | 1> =>
    Object.fromEntries(Array.from({ length: n }, (_, i) => [`src/d${i}`, 1]));
  assert.deepEqual(checkStructure(tree(dirs(MAX_DIRS)), rules), []);
  assert.equal(checkStructure(tree(dirs(MAX_DIRS + 1)), rules).length, 1);
});

it("смешанное наполнение запрещено, файл-вход и тесты — нет", () => {
  assert.equal(checkStructure(tree({ "src/a.ts": 0, "src/d": 1 }), rules).length, 1);
  assert.deepEqual(checkStructure(tree({ "src/index.ts": 0, "src/d": 1 }), rules), []);
  assert.deepEqual(checkStructure(tree({ "src/a.test.ts": 0, "src/d": 1 }), rules), []);
});

it("служебные файлы и tests/ не считаются", () => {
  const files = Object.fromEntries(["a", "b", "c", "d"].map((n) => [`src/${n}.ts`, 0]));
  const root = tree({ ...files, "src/Cargo.toml": 0, "src/tests/x.ts": 0 });
  assert.deepEqual(checkStructure(root, rules), []);
});

const meta = (root: string, pkgs: [string, string, string[]][]): CargoMetadata => ({
  packages: pkgs.map(([name, path, deps]) => ({
    name,
    manifest_path: join(root, "crates", path, "Cargo.toml"),
    dependencies: deps.map((d) => ({ name: d, kind: null })),
  })),
});

it("domain не зависит от ports, adapters зависит от ports", () => {
  const root = "/r";
  const bad = meta(root, [
    ["d", "core/domain", ["p"]],
    ["p", "core/ports", []],
  ]);
  assert.equal(checkRustLayers(root, rules, bad).length, 1);
  const ok = meta(root, [
    ["d", "core/domain", []],
    ["p", "core/ports", ["d"]],
    ["pg", "adapters/persistence/pg", ["p", "d"]],
  ]);
  assert.deepEqual(checkRustLayers(root, rules, ok), []);
});
