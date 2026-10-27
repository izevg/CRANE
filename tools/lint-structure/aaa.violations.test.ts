import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  GOOD,
  GOOD_TS,
  head,
  RS,
  rustTest,
  source,
  TS,
  tsTest,
  withIn,
  without,
  wrapped,
} from "./aaa.fixtures.ts";
import { checkAaa, checkAaaSource, findRustTests } from "./aaa.ts";

const twoActs = GOOD.map((l) =>
  l === "    let y = f(x);" ? "    let a = f(x);\n    let y = g(a);" : l,
);
const loopInAct = GOOD.map((l) =>
  l === "    let y = f(x);" ? "    for c in cases {\n        assert_eq!(f(c), 1);\n    }" : l,
);

const violating = [
  {
    name: "нет маркера Assert",
    path: RS,
    src: rustTest(without(GOOD, "// Assert")),
    expected: [`${RS}:2 sample — маркеры: // Assert ×0`],
  },
  {
    name: "маркер Act дважды",
    path: RS,
    src: rustTest(withIn(GOOD, "// Assert", ["    // Act"])),
    expected: [`${RS}:2 sample — маркеры: // Act ×2`],
  },
  {
    name: "маркер с хвостом не считается",
    path: RS,
    src: rustTest(GOOD.map((l) => (l === "    // Act" ? "    // Act: вызов" : l))),
    expected: [`${RS}:2 sample — маркеры: // Act ×0`],
  },
  {
    name: "маркеры не по порядку",
    path: RS,
    src: rustTest([
      "    // Arrange",
      "    let x = 1;",
      "    // Assert",
      "    assert_eq!(x, 1);",
      "    // Act",
      "    let y = f(x);",
    ]),
    expected: [`${RS}:2 sample — маркеры не по порядку`],
  },
  {
    name: "два оператора в Act",
    path: RS,
    src: rustTest(twoActs),
    expected: [`${RS}:2 sample — в Act операторов: 2`],
  },
  {
    name: "пустой Act",
    path: RS,
    src: rustTest(GOOD.filter((l) => l !== "    let y = f(x);")),
    expected: [`${RS}:2 sample — в Act нет оператора`],
  },
  {
    name: "assert в Arrange",
    path: RS,
    src: rustTest(withIn(GOOD, "// Act", ["    assert_eq!(x, 1);"])),
    expected: [`${RS}:2 sample — проверка до // Assert`],
  },
  {
    name: "цикл с assert внутри Act",
    path: RS,
    src: rustTest(loopInAct),
    expected: [`${RS}:2 sample — проверка до // Assert`],
  },
  {
    name: "#[should_panic]",
    path: RS,
    src: rustTest(GOOD, ["#[test]", "#[should_panic]"], ["fn sample() {"]),
    expected: [`${RS}:3 sample — #[should_panic] запрещён`],
  },
  {
    name: "тест из макроса: имя функции не распознано",
    path: RS,
    src: source("#[test]", "fn $name() {", ...GOOD, "}"),
    expected: [`${RS}:1 <тест> — тест не распознан (после атрибутов нет \`fn имя\`)`],
  },
  {
    name: "атрибут теста в несколько строк",
    path: RS,
    src: source("#[tokio::test(", '    flavor = "multi_thread"', ")]", "async fn sample() {", "}"),
    expected: [`${RS}:1 <тест> — тест не распознан (атрибут теста в нестандартной форме)`],
  },
  {
    name: "тело теста не закрыто",
    path: RS,
    src: source("#[test]", "fn sample() {", ...GOOD),
    expected: [`${RS}:2 sample — не найдено тело теста`],
  },
  {
    name: "вызван сценарий, которого нет",
    path: RS,
    src: source("pub async fn run<C>(new: impl Fn() -> C) {", "    missing(&new()).await;", "}"),
    expected: [`${RS}:2 missing — сценарий контракта без функции`],
  },
  {
    name: "сценарий контракта с двумя действиями",
    path: RS,
    src: source(
      "pub async fn run<C>(new: impl Fn() -> C) {",
      "    first(&new()).await;",
      "}",
      "async fn first(c: &C) {",
      ...twoActs,
      "}",
    ),
    expected: [`${RS}:4 first — в Act операторов: 2`],
  },
  {
    name: "привязка с лишним оператором перестаёт быть привязкой",
    path: RS,
    src: rustTest(["    let c = MemoryCatalog::default();", "    block_on(catalog_contract(c));"]),
    expected: [`${RS}:2 sample — маркеры: // Arrange ×0, // Act ×0, // Assert ×0`],
  },
  {
    name: "один оператор с assert и _contract( не считается привязкой",
    path: RS,
    src: rustTest(["    assert!(meets_contract(x));"]),
    expected: [`${RS}:2 sample — маркеры: // Arrange ×0, // Act ×0, // Assert ×0`],
  },
  {
    name: "обёртка-раннер: два оператора в Act",
    path: RS,
    src: rustTest(wrapped(twoActs.flatMap((l) => l.split("\n")))),
    expected: [`${RS}:2 sample — в Act операторов: 2`],
  },
  {
    name: "маркеры на разных уровнях",
    path: RS,
    src: rustTest(GOOD.map((l) => (l === "    // Act" ? "        // Act" : l))),
    expected: [`${RS}:2 sample — маркеры на разных уровнях`],
  },
  {
    name: "TS без маркеров",
    path: TS,
    src: tsTest(["  assert.equal(f(), 1);"]),
    expected: [`${TS}:1 sample — маркеры: // Arrange ×0, // Act ×0, // Assert ×0`],
  },
  {
    name: "TS: два оператора в Act",
    path: TS,
    src: tsTest(
      GOOD_TS.map((l) => (l === "  const y = f(x);" ? "  const a = f(x);\n  const y = g(a);" : l)),
    ),
    expected: [`${TS}:1 sample — в Act операторов: 2`],
  },
  {
    name: "TS: expect до Assert",
    path: TS,
    src: tsTest(withIn(GOOD_TS, "// Act", ["  expect(x).toBe(1);"])),
    expected: [`${TS}:1 sample — проверка до // Assert`],
  },
];

for (const c of violating) {
  it(`нарушение: ${c.name}`, () => {
    // Arrange

    // Act
    const actual = checkAaaSource(c.src, c.path).map(head);

    // Assert
    assert.deepEqual(actual, c.expected);
  });
}

it("сообщение: путь:строка имя — правило — что делать, по-русски", () => {
  // Arrange
  const src = rustTest(twoActs);

  // Act
  const actual = checkAaaSource(src, "crates/core/app/tests/yank.rs");

  // Assert
  assert.deepEqual(actual, [
    "crates/core/app/tests/yank.rs:2 sample — в Act операторов: 2 — оставьте в Act одно действие, подготовку перенесите в Arrange",
  ]);
});

it("checkAaa проверяет тест, найденный в памяти", () => {
  // Arrange
  const [found] = findRustTests(rustTest(twoActs), RS).tests;

  // Act
  const actual = found ? checkAaa(found) : ["тест не найден"];

  // Assert
  assert.equal(actual.length, 1);
});
