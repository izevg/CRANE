import assert from "node:assert/strict";
import { test as it } from "node:test";
import {
  GOOD,
  GOOD_TS,
  RS,
  rustTest,
  source,
  TS,
  tsTest,
  withIn,
  wrapped,
} from "./aaa.fixtures.ts";
import { checkAaaSource } from "./aaa.ts";

const compliant = [
  { name: "минимальный тест", path: RS, src: rustTest(GOOD) },
  {
    name: "Act цепочкой на нескольких строках",
    path: RS,
    src: rustTest(
      GOOD.map((l) =>
        l === "    let y = f(x);" ? "    let y = foo(x)\n        .bar()\n        .baz();" : l,
      ),
    ),
  },
  {
    name: "продолжение на отступе тела: . и ?",
    path: RS,
    src: rustTest(
      GOOD.map((l) => (l === "    let y = f(x);" ? "    let y = f(x)\n    .g()\n    ?;" : l)),
    ),
  },
  {
    name: "block_on с многострочным future в аргументе",
    path: RS,
    src: rustTest([
      "    // Arrange",
      "    let repo = new_repo();",
      "    // Act",
      "    let outcome = block_on(registry.publish(",
      "        coordinate(1),",
      "        payload(),",
      "    ));",
      "    // Assert",
      "    assert!(outcome.is_ok());",
    ]),
  },
  {
    name: "оператор с ? в Act",
    path: RS,
    src: rustTest(GOOD.map((l) => (l === "    let y = f(x);" ? "    let y = f(x)?;" : l))),
  },
  {
    name: "unwrap_err через несколько строк",
    path: RS,
    src: rustTest(
      GOOD.map((l) =>
        l === "    let y = f(x);" ? "    let error = block_on(run(x))\n        .unwrap_err();" : l,
      ),
    ),
  },
  {
    name: ".expect в Arrange",
    path: RS,
    src: rustTest(
      GOOD.map((l) => (l === "    let x = 1;" ? '    let x = make().expect("setup");' : l)),
    ),
  },
  {
    name: "matches! в Act",
    path: RS,
    src: rustTest(
      GOOD.map((l) => (l === "    let y = f(x);" ? "    let ok = matches!(f(x), Some(_));" : l)),
    ),
  },
  {
    name: "вложенные замыкание и fn в теле",
    path: RS,
    src: rustTest(
      withIn(GOOD, "// Act", [
        "    let double = |a: u8| {",
        "        a * 2",
        "    };",
        "    fn helper() {",
        "        touch();",
        "    }",
      ]),
    ),
  },
  {
    name: "#[tokio::test(flavor = ...)]",
    path: RS,
    src: rustTest(GOOD, ['#[tokio::test(flavor = "multi_thread")]'], ["async fn sample() {"]),
  },
  {
    name: "сигнатура на нескольких строках",
    path: RS,
    src: rustTest(GOOD, ["#[test]"], ["fn sample<C>(", "    value: C,", ") -> Result<(), E> {"]),
  },
  {
    name: "секции внутри обёртки-раннера",
    path: RS,
    src: rustTest(wrapped(GOOD)),
  },
  {
    name: "привязка адаптера к контракту без маркеров",
    path: RS,
    src: rustTest(["    block_on(catalog_contract(MemoryCatalog::default));"]),
  },
  {
    name: "сценарий контракта; хелперы файла не проверяются",
    path: "crates/core/ports/src/contract/catalog.rs",
    src: source(
      "pub async fn catalog_contract<C>(new: impl Fn() -> C) {",
      "    first(&new()).await;",
      "}",
      "",
      "async fn helper(c: &C) {",
      "    c.touch();",
      "    c.touch();",
      "}",
      "",
      "async fn first(c: &C) {",
      ...GOOD,
      "}",
    ),
  },
  { name: "минимальный TS-тест", path: TS, src: tsTest(GOOD_TS) },
  {
    name: "TS-таблица: for вокруг it",
    path: TS,
    src: source(
      "for (const c of cases) {",
      "  it(title, () => {",
      ...GOOD_TS.map((l) => `  ${l}`),
      "  });",
      "}",
    ),
  },
  {
    name: "TS: Act цепочкой на нескольких строках",
    path: TS,
    src: tsTest(
      GOOD_TS.map((l) =>
        l === "  const y = f(x);" ? "  const y = f(x)\n    .then(g)\n    .catch(h);" : l,
      ),
    ),
  },
];

for (const c of compliant) {
  it(`допустимая форма: ${c.name}`, () => {
    // Arrange

    // Act
    const actual = checkAaaSource(c.src, c.path);

    // Assert
    assert.deepEqual(actual, []);
  });
}
