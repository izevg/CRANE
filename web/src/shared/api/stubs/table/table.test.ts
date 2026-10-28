import { describe, expect, it } from "vitest";
import * as api from "../../generated/admin.gen";
import { operations } from "../../generated/operations.gen";
import { STUBS } from "./table";

const entries = Object.entries(STUBS);
const VERSION = /^v\d+\.\d+\.\d+$/;
const MARKER = /^crane-stub:\S+$/;

function queryOptionsName(operationId: string): string {
  return `get${operationId.charAt(0).toUpperCase()}${operationId.slice(1)}QueryOptions`;
}

describe("таблица заглушек", () => {
  it.each(entries)("%s — операция спеки", (operationId) => {
    // Arrange
    const known = Object.keys(operations);

    // Act
    const present = known.includes(operationId);

    // Assert
    expect(present).toBe(true);
  });

  it.each(entries)("%s — версия замены vX.Y.Z", (_id, entry) => {
    // Arrange
    const version = entry.replacedIn;

    // Act
    const valid = VERSION.test(version);

    // Assert
    expect(valid).toBe(true);
  });

  it.each(entries)("%s — есть сгенерированные опции запроса", (operationId) => {
    // Arrange
    const name = queryOptionsName(operationId);

    // Act
    const generated = Reflect.get(api, name);

    // Assert
    expect(generated).toBeTypeOf("function");
  });

  it.each(entries)("%s — модуль данных экспортирует обработчик", async (operationId, entry) => {
    // Arrange
    const load = entry.load;

    // Act
    const stub = await load?.();

    // Assert
    expect(Reflect.get(stub ?? {}, operationId)).toBeTypeOf("function");
  });

  it.each(entries)("%s — модуль данных экспортирует маркер", async (_id, entry) => {
    // Arrange
    const load = entry.load;

    // Act
    const stub = await load?.();

    // Assert
    expect(Reflect.get(stub ?? {}, "STUB_MARKER")).toMatch(MARKER);
  });
});
