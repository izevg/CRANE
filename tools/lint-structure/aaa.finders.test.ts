import assert from "node:assert/strict";
import { test as it } from "node:test";
import { GOOD_TS, RS, source, TS, tsTest } from "./aaa.fixtures.ts";
import { findContractScenarios, findRustTests, findTsTests } from "./aaa.ts";

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
