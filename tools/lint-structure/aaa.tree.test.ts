import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test as it } from "node:test";
import { GOOD, rules, rustTest, tsTest, without } from "./aaa.fixtures.ts";
import { checkAaaTree } from "./aaa.ts";

const writeTree = (files: Record<string, string>): string => {
  const root = mkdtempSync(join(tmpdir(), "aaa-"));
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(join(root, path, ".."), { recursive: true });
    writeFileSync(join(root, path), text);
  }
  return root;
};

const BAD_RS = rustTest(without(GOOD, "// Assert"));
const BAD_TS = tsTest(["  assert.equal(f(), 1);"]);
const pathOf = (violation: string): string => violation.split(" ")[0] ?? "";

it("e2e, ignoredDirs и отсутствующие корни пропускаются", () => {
  // Arrange
  const root = writeTree({
    "crates/a/tests/bad.rs": BAD_RS,
    "crates/a/e2e/bad.rs": BAD_RS,
    "crates/node_modules/bad.rs": BAD_RS,
  });

  // Act
  const actual = checkAaaTree(root, rules).map(pathOf);

  // Assert
  assert.deepEqual(actual, ["crates/a/tests/bad.rs:2"]);
});

it("stories не проверяются на AAA: это витрина, а не юнит-тесты", () => {
  // Arrange
  const root = writeTree({ "web/src/Button.stories.tsx": BAD_TS });

  // Act
  const actual = checkAaaTree(root, rules);

  // Assert
  assert.deepEqual(actual, []);
});

it("TS проверяется только в файлах *.test.*", () => {
  // Arrange
  const root = writeTree({ "web/src/bad.test.ts": BAD_TS, "web/src/helper.ts": BAD_TS });

  // Act
  const actual = checkAaaTree(root, rules).map(pathOf);

  // Assert
  assert.deepEqual(actual, ["web/src/bad.test.ts:1"]);
});
