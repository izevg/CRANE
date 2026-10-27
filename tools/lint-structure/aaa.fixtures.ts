import type { Rules } from "./checks.ts";

export const RS = "crates/core/app/tests/sample.rs";
export const TS = "tools/sample.test.ts";

export const rules: Rules = {
  scope: [],
  limits: { files: 4, dirs: 8 },
  ignoredDirs: ["node_modules"],
  unlimitedDirs: [],
  serviceFiles: [],
  testFiles: ["*.test.*"],
  storyFiles: ["*.stories.*"],
  entryFiles: [],
  rustLayers: {},
  aaa: { roots: ["crates", "web"], exemptDirs: ["e2e"] },
  size: { roots: [], exemptDirs: [], extensions: [], maxLines: 400, warnLines: 320 },
};

export const source = (...lines: string[]): string => `${lines.join("\n")}\n`;
export const rustTest = (body: string[], attrs = ["#[test]"], sig = ["fn sample() {"]): string =>
  source(...attrs, ...sig, ...body, "}");
export const tsTest = (body: string[], head = 'it("sample", () => {'): string =>
  source(head, ...body, "});");

export const GOOD = [
  "    // Arrange",
  "    let x = 1;",
  "",
  "    // Act",
  "    let y = f(x);",
  "",
  "    // Assert",
  "    assert_eq!(y, 2);",
];
export const wrapped = (lines: string[]): string[] => [
  "    support::run(async |server| {",
  ...lines.map((l) => (l === "" ? l : `    ${l}`)),
  "    });",
];
export const GOOD_TS = [
  "  // Arrange",
  "  const x = 1;",
  "  // Act",
  "  const y = f(x);",
  "  // Assert",
  "  assert.equal(y, 2);",
];
export const without = (lines: string[], marker: string): string[] =>
  lines.filter((l) => l.trim() !== marker);
export const withIn = (lines: string[], before: string, extra: string[]): string[] =>
  lines.flatMap((l) => (l.trim() === before ? [...extra, l] : [l]));

/** «path:line имя — правило» без хвоста «что делать»: сравниваем суть, а не формулировку. */
export const head = (violation: string): string => violation.split(" — ").slice(0, 2).join(" — ");
