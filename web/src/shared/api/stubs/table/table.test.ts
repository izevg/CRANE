import { describe, expect, it } from "vitest";
import * as api from "../../generated/admin.gen";
import { operations } from "../../generated/operations.gen";
import { STUB_MODULES, STUBS } from "./table";

const entries = Object.entries(STUBS);
const VERSION = /^v\d+\.\d+\.\d+$/;

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

  it.each(entries)("%s — версия замены vX.Y.Z", (_id, version) => {
    // Arrange
    const value = version;

    // Act
    const valid = VERSION.test(value);

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

  it.each(entries)("%s — какой-то модуль данных экспортирует обработчик", async (operationId) => {
    // Arrange
    const loaded = await Promise.all(Object.values(STUB_MODULES).map((load) => load()));

    // Act
    const handlers = loaded
      .map((module) => module[operationId])
      .filter((fn) => typeof fn === "function");

    // Assert
    expect(handlers).toHaveLength(1);
  });
});
