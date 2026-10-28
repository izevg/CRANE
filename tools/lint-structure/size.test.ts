import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test as it } from "node:test";
import type { Rules } from "./checks.ts";
import { lineCount, type Measured, measure, oversized, sizeReport } from "./size.ts";

const MAX = 400;
const WARN = 320;
const S10 = 10;
const S50 = 50;
const S100 = 100;
const S200 = 200;
const S300 = 300;
const S500 = 500;
const S900 = 900;
const TWO = 2;
const THREE = 3;

const rules: Rules = {
  scope: [],
  limits: { files: 4, dirs: 8 },
  ignoredDirs: ["node_modules"],
  unlimitedDirs: [],
  serviceFiles: [],
  testFiles: [],
  storyFiles: [],
  entryFiles: [],
  rustLayers: {},
  serde: { crates: [], layers: [] },
  aaa: { roots: [], exemptDirs: [] },
  size: {
    roots: ["crates", "web/src"],
    exemptDirs: ["generated", "migrations"],
    extensions: [".rs", ".ts"],
    maxLines: MAX,
    warnLines: WARN,
  },
};

const file = (path: string, lines: number): Measured => ({ path, lines });
const rows = (n: number): string => "x\n".repeat(n);

const writeTree = (files: Record<string, string>): string => {
  const root = mkdtempSync(join(tmpdir(), "size-"));
  for (const [path, text] of Object.entries(files)) {
    mkdirSync(join(root, path, ".."), { recursive: true });
    writeFileSync(join(root, path), text);
  }
  return root;
};

const lineCases = [
  { name: "пустой текст", text: "", expected: 0 },
  { name: "последняя строка с переводом", text: "a\nb\n", expected: 2 },
  { name: "последняя строка без перевода считается", text: "a\nb", expected: 2 },
  { name: "одна строка без перевода", text: "a", expected: 1 },
  { name: "только перевод строки", text: "\n", expected: 1 },
  { name: "CRLF даёт одну строку на каждый \\n", text: "a\r\nb\r\n", expected: 2 },
];

for (const c of lineCases) {
  it(`lineCount: ${c.name}`, () => {
    // Arrange
    const { text } = c;

    // Act
    const actual = lineCount(text);

    // Assert
    assert.equal(actual, c.expected);
  });
}

it("oversized: файл ровно на лимите проходит", () => {
  // Arrange
  const files = [file("a.rs", MAX)];

  // Act
  const actual = oversized(files, MAX);

  // Assert
  assert.deepEqual(actual, []);
});

it("oversized: файл на строку длиннее лимита нарушает", () => {
  // Arrange
  const files = [file("a.rs", MAX + 1)];

  // Act
  const actual = oversized(files, MAX);

  // Assert
  assert.deepEqual(actual, ["a.rs: 401 строк при лимите 400"]);
});

it("oversized: нарушители по убыванию размера, при равенстве по пути", () => {
  // Arrange
  const files = [file("b.rs", S500), file("small.rs", S10), file("c.rs", S900), file("a.rs", S500)];

  // Act
  const actual = oversized(files, MAX);

  // Assert
  assert.deepEqual(actual, [
    "c.rs: 900 строк при лимите 400",
    "a.rs: 500 строк при лимите 400",
    "b.rs: 500 строк при лимите 400",
  ]);
});

it("sizeReport: показывает top самых больших, остальное ниже порога скрыто", () => {
  // Arrange
  const files = [file("a", S100), file("b", S300), file("c", S200), file("d", S50)];
  const top = 2;

  // Act
  const actual = sizeReport(files, top, WARN, MAX).slice(1);

  // Assert
  assert.deepEqual(actual, ["300\tb", "200\tc"]);
});

it("sizeReport: порог предупреждения включительно, сверх top", () => {
  // Arrange
  const files = [file("a", WARN), file("b", WARN - 1), file("c", MAX + 1), file("d", MAX)];

  // Act
  const actual = sizeReport(files, 0, WARN, MAX).slice(1);

  // Assert
  assert.deepEqual(actual, [
    "401\tc\tнарушение",
    "400\td\tпредупреждение",
    "320\ta\tпредупреждение",
  ]);
});

it("sizeReport: заголовок сводит число файлов, нарушений и предупреждений", () => {
  // Arrange
  const files = [file("a", MAX + 1), file("b", MAX), file("c", 1)];

  // Act
  const actual = sizeReport(files, 0, WARN, MAX)[0];

  // Assert
  assert.equal(actual, "Файлов: 3; нарушений (> 400): 1; предупреждений (>= 320): 1");
});

it("measure: считает строки подходящих файлов", () => {
  // Arrange
  const root = writeTree({ "crates/a/lib.rs": rows(THREE), "web/src/b.ts": "one\ntwo" });

  // Act
  const actual = measure(root, rules);

  // Assert
  assert.deepEqual(actual, [file("crates/a/lib.rs", THREE), file("web/src/b.ts", TWO)]);
});

it("measure: generated, migrations, ignoredDirs и точечные папки пропускаются", () => {
  // Arrange
  const root = writeTree({
    "crates/a/lib.rs": rows(1),
    "crates/a/generated/g.rs": rows(1),
    "crates/a/migrations/m.rs": rows(1),
    "crates/node_modules/n.rs": rows(1),
    "crates/.hidden/h.rs": rows(1),
    "web/src/shared/api/generated/g.ts": rows(1),
  });

  // Act
  const actual = measure(root, rules).map((m) => m.path);

  // Assert
  assert.deepEqual(actual, ["crates/a/lib.rs"]);
});

it("measure: чужое расширение и отсутствующий корень игнорируются", () => {
  // Arrange
  const root = writeTree({ "crates/a/notes.md": rows(1), "crates/a/lib.rs": rows(1) });

  // Act
  const actual = measure(root, rules).map((m) => m.path);

  // Assert
  assert.deepEqual(actual, ["crates/a/lib.rs"]);
});
