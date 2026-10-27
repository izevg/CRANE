import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { test as it } from "node:test";
import { checkSource, checkTree } from "./checks.ts";

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

it("файлы данных витрины проверяются, а контент в них помечается i18n-fixture", () => {
  // Arrange
  const marked = '// i18n-fixture: имена людей — контент витрины\nexport const a = "Иванов И.";\n';
  const root = writeTree({
    "web/src/a.ts": "export const a = 1;\n",
    "web/screens/data/people.ts": 'export const a = "Иванов И.";\n',
    "web/screens/data/marked.ts": marked,
  });

  // Act
  const actual = checkTree(root).map((v) => v.split(":")[0]);

  // Assert
  assert.deepEqual(actual, ["web/screens/data/people.ts"]);
});

it("i18n-fixture без причины не снимает нарушение", () => {
  // Arrange
  const code = source("// i18n-fixture:", 'export const a = "Иванов И.";');

  // Act
  const actual = checkSource(code, FILE);

  // Assert
  assert.equal(actual.length, 1);
});

it("тело play и title/name истории — служебные части stories, не интерфейс", () => {
  // Arrange
  const code = source(
    'const meta = { title: "Экраны/S1 Поиск", component: C } satisfies Meta<typeof C>;',
    'export const A: Story = { name: "Десктоп", play: async () => { await expect(x).toHaveText("Закрыть"); } };',
  );

  // Act
  const actual = checkSource(code, "web/screens/s1/S1.stories.tsx");

  // Assert
  assert.deepEqual(actual, []);
});

it("name вне истории (подпись колонки) остаётся нарушением", () => {
  // Arrange
  const code = source('const columns = { name: "Версия" };');

  // Act
  const actual = checkSource(code, "web/screens/s1/S1.stories.tsx");

  // Assert
  assert.equal(actual.length, 1);
});

it("объявление с i18n-fixture и причиной — фикстура, пропускается целиком", () => {
  // Arrange
  const code = source(
    "// i18n-fixture: имена команд — контент витрины",
    'const TEAMS = ["Платформа", "Платежи"];',
    'const UI = "Закрыть";',
  );

  // Act
  const actual = checkSource(code, "web/screens/s1/S1.stories.tsx");

  // Assert
  assert.equal(actual.length, 1);
});
