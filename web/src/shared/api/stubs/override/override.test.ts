import { afterEach, describe, expect, it } from "vitest";
import { applyOverrides, override, overrideFor, resetOverrides } from "./override";

afterEach(() => {
  resetOverrides();
});

describe("подмены Storybook", () => {
  it("override регистрирует обработчик операции", () => {
    // Arrange
    const handler = () => ({ ok: true });
    override("getStatus", handler);

    // Act
    const found = overrideFor("getStatus");

    // Assert
    expect(found).toBe(handler);
  });

  it("applyOverrides сбрасывает подмены прошлой истории и регистрирует новые", () => {
    // Arrange
    override("getMe", () => "прошлая история");
    const next = () => "эта история";

    // Act
    applyOverrides({ getStatus: next });

    // Assert
    expect(overrideFor("getStatus")).toBe(next);
    expect(overrideFor("getMe")).toBeUndefined();
  });

  it("история без parameters.api оставляет операции без подмен", () => {
    // Arrange
    override("getMe", () => "прошлая история");

    // Act
    applyOverrides(undefined);

    // Assert
    expect(overrideFor("getMe")).toBeUndefined();
  });

  it("parameters.api = null не падает и тоже сбрасывает подмены", () => {
    // Arrange
    override("getMe", () => "прошлая история");

    // Act
    applyOverrides(null);

    // Assert
    expect(overrideFor("getMe")).toBeUndefined();
  });

  it("не функция вместо обработчика — ошибка истории, а не потеря подмены", () => {
    // Arrange
    const parameters = { getStatus: { version: "1" } };

    // Act
    const apply = () => applyOverrides(parameters);

    // Assert
    expect(apply).toThrow("getStatus");
  });
});
