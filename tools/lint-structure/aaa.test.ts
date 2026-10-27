import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test as it } from "node:test";
import {
  checkAaa,
  checkAaaSource,
  checkAaaTree,
  findContractScenarios,
  findRustTests,
  findTsTests,
} from "./aaa.ts";
import type { Rules } from "./checks.ts";

const RS = "crates/core/app/tests/sample.rs";
const TS = "tools/sample.test.ts";

const rules: Rules = {
  scope: [],
  limits: { files: 4, dirs: 8 },
  ignoredDirs: ["node_modules"],
  unlimitedDirs: [],
  serviceFiles: [],
  testFiles: ["*.test.*"],
  entryFiles: [],
  rustLayers: {},
  aaa: { roots: ["crates", "web"], exemptDirs: ["e2e"] },
};

const source = (...lines: string[]): string => `${lines.join("\n")}\n`;
const rustTest = (body: string[], attrs = ["#[test]"], sig = ["fn sample() {"]): string =>
  source(...attrs, ...sig, ...body, "}");
const tsTest = (body: string[], head = 'it("sample", () => {'): string =>
  source(head, ...body, "});");

const GOOD = [
  "    // Arrange",
  "    let x = 1;",
  "",
  "    // Act",
  "    let y = f(x);",
  "",
  "    // Assert",
  "    assert_eq!(y, 2);",
];
const wrapped = (lines: string[]): string[] => [
  "    support::run(async |server| {",
  ...lines.map((l) => (l === "" ? l : `    ${l}`)),
  "    });",
];
const GOOD_TS = [
  "  // Arrange",
  "  const x = 1;",
  "  // Act",
  "  const y = f(x);",
  "  // Assert",
  "  assert.equal(y, 2);",
];
const without = (lines: string[], marker: string): string[] =>
  lines.filter((l) => l.trim() !== marker);
const withIn = (lines: string[], before: string, extra: string[]): string[] =>
  lines.flatMap((l) => (l.trim() === before ? [...extra, l] : [l]));

/** «path:line имя — правило» без хвоста «что делать»: сравниваем суть, а не формулировку. */
const head = (violation: string): string => violation.split(" — ").slice(0, 2).join(" — ");

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

it("#[cfg(test)] и модуль с хелперами не считаются тестами", () => {
  // Arrange
  const src = source("#[cfg(test)]", "mod tests {", "    fn helper() {", "    }", "}");

  // Act
  const actual = findRustTests(src, RS);

  // Assert
  assert.deepEqual(actual, { tests: [], violations: [] });
});

it("сценарий контракта ищется по вызову в раннере, имя берётся один раз", () => {
  // Arrange
  const src = source(
    "pub async fn run<C>(new: impl Fn() -> C) {",
    "    first(&new()).await;",
    "    first(&new()).await;",
    "}",
    "async fn first(c: &C) {",
    "}",
  );

  // Act
  const actual = findContractScenarios(src, RS).tests.map((t) => t.name);

  // Assert
  assert.deepEqual(actual, ["first"]);
});

const tsForms = [
  { form: "it", head: 'it("имя", () => {', expected: "имя" },
  { form: "test", head: 'test("имя", async () => {', expected: "имя" },
  { form: "it.only", head: 'it.only("имя", () => {', expected: "имя" },
  { form: "it.skip", head: 'it.skip("имя", () => {', expected: "имя" },
  { form: "it.each", head: 'it.each(cases)("имя %s", (c) => {', expected: "имя %s" },
  { form: "заголовок на следующей строке", head: "it(\n  'имя',\n  () => {", expected: "имя" },
];

for (const c of tsForms) {
  it(`TS: форма ${c.form} распознаётся`, () => {
    // Arrange
    const src = tsTest(GOOD_TS, c.head);

    // Act
    const actual = findTsTests(src, TS).tests.map((t) => t.name);

    // Assert
    assert.deepEqual(actual, [c.expected]);
  });
}

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

it("TS проверяется только в файлах *.test.*", () => {
  // Arrange
  const root = writeTree({ "web/src/bad.test.ts": BAD_TS, "web/src/helper.ts": BAD_TS });

  // Act
  const actual = checkAaaTree(root, rules).map(pathOf);

  // Assert
  assert.deepEqual(actual, ["web/src/bad.test.ts:1"]);
});
