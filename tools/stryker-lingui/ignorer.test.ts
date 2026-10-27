import assert from "node:assert/strict";
import { test as it } from "node:test";
import { shouldIgnore, strykerPlugins } from "./ignorer.ts";

const tagged = (name: string) => ({
  node: { type: "TaggedTemplateExpression", tag: { type: "Identifier", name } },
});
const call = (name: string) => ({
  node: { type: "CallExpression", callee: { type: "Identifier", name } },
});

it("шаблон с тегом t или msg игнорируется", () => {
  // Arrange
  const nodes = [tagged("t"), tagged("msg")];

  // Act
  const actual = nodes.map((path) => shouldIgnore(path) !== undefined);

  // Assert
  assert.deepEqual(actual, [true, true]);
});

it("вызов plural и select игнорируется", () => {
  // Arrange
  const nodes = [call("plural"), call("select")];

  // Act
  const actual = nodes.map((path) => shouldIgnore(path) !== undefined);

  // Assert
  assert.deepEqual(actual, [true, true]);
});

it("чужие вызовы и теги мутируются", () => {
  // Arrange
  const nodes = [tagged("html"), call("format"), { node: { type: "StringLiteral" } }];

  // Act
  const actual = nodes.map((path) => shouldIgnore(path));

  // Assert
  assert.deepEqual(actual, [undefined, undefined, undefined]);
});

it("тело функции с useLingui игнорируется, тело без него мутируется", () => {
  // Arrange
  const withHook = {
    node: {
      type: "BlockStatement",
      body: [
        {
          type: "VariableDeclaration",
          declarations: [
            { init: { type: "CallExpression", callee: { type: "Identifier", name: "useLingui" } } },
          ],
        },
      ],
    },
  };
  const without = { node: { type: "BlockStatement", body: [{ type: "ReturnStatement" }] } };

  // Act
  const actual = [withHook, without].map((path) => shouldIgnore(path) !== undefined);

  // Assert
  assert.deepEqual(actual, [true, false]);
});

it("плагин объявлен как игнорер с именем lingui-macros", () => {
  // Arrange
  const [plugin] = strykerPlugins;

  // Act
  const actual = { kind: plugin?.kind, name: plugin?.name };

  // Assert
  assert.deepEqual(actual, { kind: "Ignore", name: "lingui-macros" });
});
