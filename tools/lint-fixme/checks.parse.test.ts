import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  indexOperations,
  parseFixme,
  parseGrep,
  parseMilestones,
  parseOperations,
  withNext,
} from "./checks.ts";

const NO_SINCE = /admin\.yaml: у операции listTokens нет x-crane-since/;
const FILE = "web/src/shared/api/a.ts";

it("parseFixme разбирает версию и текст", () => {
  // Arrange
  const hit = { file: FILE, line: 3, text: "  // FIXME(v0.4.0): getMe настоящий запрос" };

  // Act
  const actual = parseFixme(hit);

  // Assert
  assert.deepEqual(actual, {
    file: FILE,
    line: 3,
    version: "v0.4.0",
    text: "getMe настоящий запрос",
  });
});

const INVALID = [
  "// FIXME(v0.4): x",
  "// FIXME (v0.4.0): x",
  "// FIXME(v0.4.0) x",
  "// FIXME(v0.4.0):",
  "// FIXME(v0.4.0):   ",
  "// FIXME",
  "// FIXME: x",
  "// FIXME(settings-api): x",
];
for (const text of INVALID) {
  it(`parseFixme отвергает «${text}» с file:line`, () => {
    // Arrange
    const hit = { file: FILE, line: 9, text };

    // Act
    const actual = parseFixme(hit);

    // Assert
    assert.ok(String(actual).startsWith(`${FILE}:9: FIXME не по формату`));
  });
}

for (const marker of ["TODO", "XXX", "HACK"]) {
  it(`parseFixme отвергает маркер ${marker} и называет замену`, () => {
    // Arrange
    const hit = { file: FILE, line: 5, text: `// ${marker}: потом` };

    // Act
    const actual = parseFixme(hit);

    // Assert
    assert.equal(
      actual,
      `${FILE}:5: маркер ${marker} запрещён: замените на FIXME(vX.Y.Z): что осталось доделать из реестра или уберите`,
    );
  });
}

it("parseFixme не считает словом TODO внутри идентификатора", () => {
  // Arrange
  const hit = { file: FILE, line: 6, text: "const TODOS_COUNT = 1; // FIXME(v0.4.0): todoList" };

  // Act
  const actual = parseFixme(hit);

  // Assert
  assert.deepEqual(actual, { file: FILE, line: 6, version: "v0.4.0", text: "todoList" });
});

it("parseGrep оставляет двоеточия в тексте строки", () => {
  // Arrange
  const stdout = "a.ts:12:// FIXME(v0.4.0): x: y\nb.rs:3:// FIXME\n";

  // Act
  const actual = parseGrep(stdout);

  // Assert
  assert.deepEqual(actual, [
    { file: "a.ts", line: 12, text: "// FIXME(v0.4.0): x: y" },
    { file: "b.rs", line: 3, text: "// FIXME" },
  ]);
});

const ROADMAP = [
  "| v9.9.9 | до раздела | x |",
  "## Вехи",
  "",
  "| Версия | Содержание |",
  "| --- | --- |",
  "| v0.2.0 | Прокси | 28 октября (готово) |",
  "| v0.3.0 | PyPI | 9 ноября |",
  "| **v0.5.0** | **Полное ядро** | **20 ноября** |",
  "| v0.4.0 | Доступ | 13 ноября |",
  "",
  "## Шаг 0",
  "| v8.8.8 | после раздела | x |",
].join("\n");

it("parseMilestones берёт таблицу «Вехи» по порядку строк, с жирными версиями и done", () => {
  // Arrange
  const roadmap = ROADMAP;

  // Act
  const actual = parseMilestones(roadmap);

  // Assert
  assert.deepEqual(actual, [
    { version: "v0.2.0", done: true },
    { version: "v0.3.0", done: false },
    { version: "v0.5.0", done: false },
    { version: "v0.4.0", done: false },
  ]);
});

const YAML = [
  "      operationId: getMe",
  "      x-crane-phase: P0",
  "      x-crane-since: v0.4.0",
  "      operationId: listTokens",
  "      summary: нет since",
].join("\n");

it("parseOperations находит since операции", () => {
  // Arrange
  const yaml = YAML;

  // Act
  const actual = parseOperations(yaml, "admin.yaml");

  // Assert
  assert.deepEqual(actual[0], { id: "getMe", since: "v0.4.0", source: "admin.yaml" });
});

it("parseOperations не берёт since соседней операции", () => {
  // Arrange
  const yaml = YAML;

  // Act
  const actual = parseOperations(yaml, "admin.yaml");

  // Assert
  assert.deepEqual(actual[1], { id: "listTokens", since: undefined, source: "admin.yaml" });
});

it("indexOperations строит индекс id -> since", () => {
  // Arrange
  const operations = [{ id: "a", since: "v0.3.0", source: "admin.yaml" }];

  // Act
  const actual = indexOperations(operations);

  // Assert
  assert.deepEqual([[...actual.since], actual.errors], [[["a", "v0.3.0"]], []]);
});

it("indexOperations сообщает об операции без since с источником", () => {
  // Arrange
  const operations = [{ id: "listTokens", since: undefined, source: "admin.yaml" }];

  // Act
  const actual = indexOperations(operations);

  // Assert
  assert.match(actual.errors.join(), NO_SINCE);
});

const DUPLICATES = [
  ["внутри файла", "admin.yaml", "admin.yaml"],
  ["между файлами", "admin.yaml", "common.yaml"],
] as const;
for (const [name, first, second] of DUPLICATES) {
  it(`indexOperations: дубликат ${name} - одно нарушение с обоими источниками`, () => {
    // Arrange
    const operations = [
      { id: "a", since: "v0.3.0", source: first },
      { id: "a", since: "v0.3.0", source: second },
    ];

    // Act
    const actual = indexOperations(operations);

    // Assert
    assert.deepEqual(actual.errors, [
      `operationId a повторяется (${first} и ${second}). operationId должен быть уникален`,
    ]);
  });
}

it("withNext пропускает комментарии и пустые строки до записи", () => {
  // Arrange
  const stub = { file: FILE, line: 2, version: "v0.4.0", text: "getMe x" };
  const source = [
    "",
    "// FIXME(v0.4.0): getMe x",
    "",
    "// ещё пояснение",
    "/* блок",
    " * строка",
    " */",
    'getMe: { replacedIn: "v0.4.0" },',
  ].join("\n");

  // Act
  const actual = withNext(stub, source);

  // Assert
  assert.equal(actual.next, 'getMe: { replacedIn: "v0.4.0" },');
});

it("withNext не добавляет next, если после FIXME ничего нет", () => {
  // Arrange
  const stub = { file: FILE, line: 1, version: "v0.4.0", text: "getMe x" };

  // Act
  const actual = withNext(stub, "// FIXME(v0.4.0): getMe x\n\n");

  // Assert
  assert.deepEqual(actual, stub);
});
