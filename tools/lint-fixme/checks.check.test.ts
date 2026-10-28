import assert from "node:assert/strict";
import { test as it } from "node:test";
import { fixme, MILESTONES, OPERATIONS, STUB_NEXT } from "./checks.fixtures.ts";
import { check, type Fixme, report } from "./checks.ts";

const CLOSED = /crates\/app\/src\/lib\.rs:7: веха v0\.2\.0 закрыта, заглушка не снята/;
const UNKNOWN_MILESTONE = /веха v7\.0\.0 не найдена/;
const OUTSIDE_API = /допустим только под web\/src\/shared\/api\//;
const UNKNOWN_OPERATION = /«noSuchOp» не operationId/;
const BEFORE_SINCE = /веха v0\.3\.0 раньше x-crane-since v0\.4\.0/;
const NO_SINCE = /admin\.yaml: у операции listTokens нет x-crane-since/;
const API = "web/src/shared/api/steps.ts";

it("FIXME на закрытую веху — нарушение", () => {
  // Arrange
  const stubs = [fixme({ version: "v0.2.0" })];

  // Act
  const actual = check(stubs, MILESTONES, OPERATIONS);

  // Assert
  assert.match(actual.join(), CLOSED);
});

it("FIXME на веху вне таблицы — нарушение", () => {
  // Arrange
  const stubs = [fixme({ version: "v7.0.0" })];

  // Act
  const actual = check(stubs, MILESTONES, OPERATIONS);

  // Assert
  assert.match(actual.join(), UNKNOWN_MILESTONE);
});

it("Rust FIXME с произвольным текстом на открытую веху — ок", () => {
  // Arrange
  const stubs = [fixme({ text: "любой текст без operationId" })];

  // Act
  const actual = check(stubs, MILESTONES, OPERATIONS);

  // Assert
  assert.deepEqual(actual, []);
});

it("FIXME в web/src вне shared/api — нарушение", () => {
  // Arrange
  const stubs = [fixme({ file: "web/src/pages/home/ui.tsx", text: "getMe" })];

  // Act
  const actual = check(stubs, MILESTONES, OPERATIONS);

  // Assert
  assert.match(actual.join(), OUTSIDE_API);
});

it("неизвестный operationId в shared/api — нарушение", () => {
  // Arrange
  const stubs = [fixme({ file: API, text: "noSuchOp шаг" })];

  // Act
  const actual = check(stubs, MILESTONES, OPERATIONS);

  // Assert
  assert.match(actual.join(), UNKNOWN_OPERATION);
});

it("веха FIXME раньше x-crane-since операции — нарушение", () => {
  // Arrange
  const stubs = [fixme({ file: API, version: "v0.3.0", text: "getMe шаг" })];

  // Act
  const actual = check(stubs, MILESTONES, OPERATIONS);

  // Assert
  assert.match(actual.join(), BEFORE_SINCE);
});

for (const version of ["v0.4.0", "v0.5.0"]) {
  it(`веха ${version} не раньше x-crane-since — ок`, () => {
    // Arrange
    const stubs = [
      fixme({
        file: API,
        version,
        text: "getMe шаг",
        next: `getMe: { replacedIn: "${version}" },`,
      }),
    ];

    // Act
    const actual = check(stubs, MILESTONES, OPERATIONS);

    // Assert
    assert.deepEqual(actual, []);
  });
}

const MISMATCH = /версия FIXME не совпадает с записью заглушки/;
const apiStub = (next?: string) =>
  fixme({
    file: API,
    version: "v0.4.0",
    text: "getMe шаг",
    ...(next === undefined ? {} : { next }),
  });

it("версия в записи заглушки совпадает с FIXME — ок", () => {
  // Arrange
  const stubs = [apiStub(STUB_NEXT)];

  // Act
  const actual = check(stubs, MILESTONES, OPERATIONS);

  // Assert
  assert.deepEqual(actual, []);
});

it("другая версия в записи заглушки — нарушение", () => {
  // Arrange
  const stubs = [apiStub('getMe: { replacedIn: "v0.5.0" },')];

  // Act
  const actual = check(stubs, MILESTONES, OPERATIONS);

  // Assert
  assert.match(actual.join(), MISMATCH);
});

it("без записи заглушки после FIXME (конец файла) — нарушение", () => {
  // Arrange
  const stubs = [apiStub()];

  // Act
  const actual = check(stubs, MILESTONES, OPERATIONS);

  // Assert
  assert.match(actual.join(), MISMATCH);
});

it("ошибки разбора контракта попадают в нарушения", () => {
  // Arrange
  const operations = [{ id: "listTokens", since: undefined, source: "admin.yaml" }];

  // Act
  const actual = check([], MILESTONES, operations);

  // Assert
  assert.match(actual.join(), NO_SINCE);
});

it("report группирует по порядку вех, а не по алфавиту", () => {
  // Arrange
  const stubs = [
    fixme({ version: "v0.5.0", line: 1, text: "поздняя" }),
    fixme({ version: "v0.3.0", line: 2, text: "ранняя" }),
  ];

  // Act
  const actual = report(stubs, [], MILESTONES);

  // Assert
  assert.deepEqual(actual, [
    "v0.3.0: 1",
    "  crates/app/src/lib.rs:2  ранняя",
    "v0.5.0: 1",
    "  crates/app/src/lib.rs:1  поздняя",
  ]);
});

it("report выводит нарушения формата блоком «без вехи»", () => {
  // Arrange
  const invalid = ["a.ts:4: FIXME не по формату"];

  // Act
  const actual = report([], invalid, MILESTONES);

  // Assert
  assert.deepEqual(actual, ["без вехи: 1", "  a.ts:4: FIXME не по формату"]);
});

it("report пустого реестра сообщает, что заглушек нет", () => {
  // Arrange
  const stubs: Fixme[] = [];

  // Act
  const actual = report(stubs, [], MILESTONES);

  // Assert
  assert.deepEqual(actual, ["заглушек нет"]);
});
