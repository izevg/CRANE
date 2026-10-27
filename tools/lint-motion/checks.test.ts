import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { test as it } from "node:test";
import { checkSource, checkTree } from "./checks.ts";

const FILE = "web/src/shared/ui/Sample.tsx";
const source = (...lines: string[]): string => `${lines.join("\n")}\n`;

it("transition-all в классе — нарушение", () => {
  // Arrange
  const code = source('export const a = "p-2 transition-all duration-(--dur-fast)";');

  // Act
  const actual = checkSource(code, FILE);

  // Assert
  assert.equal(actual.length, 1);
});

it("голый transition — нарушение, а transition-colors и transition-none — нет", () => {
  // Arrange
  const bad = source('export const a = "flex transition";');
  const good = source('export const a = "transition-colors transition-none transition-opacity";');

  // Act
  const actual = [checkSource(bad, FILE).length, checkSource(good, FILE).length];

  // Assert
  assert.deepEqual(actual, [1, 0]);
});

it("голый transition под вариантом hover: тоже нарушение", () => {
  // Arrange
  const code = source('export const a = "hover:transition md:transition";');

  // Act
  const actual = checkSource(code, FILE);

  // Assert
  assert.equal(actual.length, 2);
});

it("transition-[…] со свойствами раскладки — нарушение, с transform и opacity — нет", () => {
  // Arrange
  const bad = source(
    'export const a = "transition-[translate,width,height] transition-[margin-left]";',
  );
  const good = source(
    'export const a = "transition-[opacity,translate] transition-[background-color,color,transform]";',
  );

  // Act
  const actual = [checkSource(bad, FILE).length, checkSource(good, FILE).length];

  // Assert
  assert.deepEqual(actual, [2, 0]);
});

it("duration числом — нарушение, токен — нет", () => {
  // Arrange
  const bad = source('export const a = "duration-200 duration-[300ms]";');
  const good = source('export const a = "duration-(--dur-fast) md:duration-(--dur-base)";');

  // Act
  const actual = [checkSource(bad, FILE).length, checkSource(good, FILE).length];

  // Assert
  assert.deepEqual(actual, [2, 0]);
});

it("transition: all в inline-стиле и в CSS — нарушение", () => {
  // Arrange
  const ts = source('const style = "transition: all 200ms";');
  const css = source(".a {", "  transition: all 0.2s;", "}");

  // Act
  const actual = [checkSource(ts, FILE).length, checkSource(css, "web/src/app/a.css").length];

  // Assert
  assert.deepEqual(actual, [1, 1]);
});

it("в CSS допустимы явные свойства, токены и нулевая длительность", () => {
  // Arrange
  const css = source(
    ".a {",
    "  transition-property: opacity, transform;",
    "  transition-duration: var(--dur-fast);",
    "  transition-duration: 0s;",
    "  transition: none;",
    "}",
  );

  // Act
  const actual = checkSource(css, "web/src/app/a.css");

  // Assert
  assert.deepEqual(actual, []);
});

it("в CSS числовая длительность перехода — нарушение", () => {
  // Arrange
  const css = source(".a {", "  transition-duration: 200ms;", "}");

  // Act
  const actual = checkSource(css, "web/src/app/a.css");

  // Assert
  assert.equal(actual.length, 1);
});

it("слова про transition в комментариях и тексте не считаются", () => {
  // Arrange
  const code = source("// transition-all тут только упомянут", "const note = 1;");

  // Act
  const actual = checkSource(code, FILE);

  // Assert
  assert.deepEqual(actual, []);
});

it("motion-ignore с причиной снимает нарушение, без причины — нет", () => {
  // Arrange
  const withReason = source(
    "// motion-ignore: библиотека тостов сама держит траекторию",
    'const a = "transition-all";',
  );
  const noReason = source("// motion-ignore:", 'const a = "transition-all";');

  // Act
  const actual = [checkSource(withReason, FILE).length, checkSource(noReason, FILE).length];

  // Assert
  assert.deepEqual(actual, [0, 1]);
});

it("checkTree проверяет web/src и web/screens, но не тесты", () => {
  // Arrange
  const root = mkdtempSync(join(tmpdir(), "lint-motion-"));
  const bad = 'export const a = "transition";\n';
  for (const rel of [
    "web/src/a.tsx",
    "web/src/a.test.tsx",
    "web/screens/s1/S1.stories.tsx",
    "web/other/o.tsx",
  ]) {
    const full = join(root, rel);
    mkdirSync(dirname(full), { recursive: true });
    writeFileSync(full, bad);
  }

  // Act
  const actual = checkTree(root).map((message) => message.split(":")[0] ?? "");

  // Assert
  assert.deepEqual(
    actual.sort((a, b) => a.localeCompare(b)),
    ["web/screens/s1/S1.stories.tsx", "web/src/a.tsx"],
  );
});
