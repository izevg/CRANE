import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { test as it } from "node:test";
import { checkSource, checkTree, MARKER } from "./checks.ts";

const FILE = "web/src/shared/ui/Sample.tsx";
const source = (...lines: string[]): string => `${lines.join("\n")}\n`;

it("кириллический литерал в JSX и в строке — нарушение", () => {
  // Arrange
  const code = source('const A = "Закрыть";', "export const B = () => <p>Привет</p>;");

  // Act
  const actual = checkSource(code, FILE);

  // Assert
  assert.equal(actual.length, 2);
});

it("текст внутри t, msg, plural и Trans — не нарушение", () => {
  // Arrange
  const code = source(
    "const a = t`Закрыть`;",
    "const b = msg`Открыть`;",
    'const c = plural(n, { one: "# файл", other: "# файлов" });',
    "const d = <Trans>Привет</Trans>;",
    'const e = t({ message: "Закрыть", comment: "кнопка" });',
  );

  // Act
  const actual = checkSource(code, FILE);

  // Assert
  assert.deepEqual(actual, []);
});

it("сообщение throw new Error и импорты не считаются", () => {
  // Arrange
  const code = source('throw new Error("Корневой элемент не найден");');

  // Act
  const actual = checkSource(code, FILE);

  // Assert
  assert.deepEqual(actual, []);
});

it("комментарии и латиница не считаются", () => {
  // Arrange
  const code = source("// Комментарий по-русски", 'const a = "close";');

  // Act
  const actual = checkSource(code, FILE);

  // Assert
  assert.deepEqual(actual, []);
});

it("i18n-ignore с причиной снимает нарушение на строке", () => {
  // Arrange
  const code = source("// i18n-ignore: эндоним языка", 'const a = "Русский";');

  // Act
  const actual = checkSource(code, FILE);

  // Assert
  assert.deepEqual(actual, []);
});

it("маркер немигрированного файла пропускает его литералы", () => {
  // Arrange
  const code = source(MARKER, 'const a = "Закрыть";');

  // Act
  const actual = checkSource(code, FILE);

  // Assert
  assert.deepEqual(actual, []);
});

it("маркер в файле без литералов — нарушение: его нужно удалить", () => {
  // Arrange
  const code = source(MARKER, "const a = 1;");

  // Act
  const actual = checkSource(code, FILE);

  // Assert
  assert.equal(actual.length, 1);
});

const writeTree = (files: Record<string, string>): string => {
  const root = mkdtempSync(join(tmpdir(), "li18n-"));
  for (const [path, content] of Object.entries(files)) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), content);
  }
  return root;
};

it("тесты и stories в src не проверяются, а экраны в web/screens — проверяются", () => {
  // Arrange
  const bad = 'export const a = "Закрыть";\n';
  const root = writeTree({
    "web/src/a.test.ts": bad,
    "web/src/a.stories.tsx": bad,
    "web/src/a.ts": "export const a = 1;\n",
    "web/screens/s1/S1.stories.tsx": bad,
  });

  // Act
  const actual = checkTree(root).map((v) => v.split(":")[0]);

  // Assert
  assert.deepEqual(actual, ["web/screens/s1/S1.stories.tsx"]);
});
