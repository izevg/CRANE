import { afterEach, describe, expect, it } from "vitest";
import { readStored, removeStored, writeStored } from "./storage";

const isNumber = (value: unknown): value is number => typeof value === "number";
const isString = (value: unknown): value is string => typeof value === "string";
const KEY = "crane.test";

function brokenStorage(): Storage {
  const fail = (): never => {
    throw new Error("storage недоступен");
  };
  return { getItem: fail, setItem: fail, removeItem: fail, clear: fail, key: fail, length: 0 };
}

afterEach(() => {
  localStorage.clear();
  sessionStorage.clear();
});

describe("readStored", () => {
  it("читает записанное значение из выбранного хранилища", () => {
    // Arrange
    writeStored("session", KEY, { draft: "текст" });
    const isDraft = (value: unknown): value is { draft: string } =>
      typeof value === "object" && value !== null && "draft" in value;

    // Act
    const actual = readStored("session", KEY, isDraft);

    // Assert
    expect(actual).toEqual({ draft: "текст" });
  });

  it("не путает local и session", () => {
    // Arrange
    writeStored("local", KEY, "в local");

    // Act
    const actual = readStored("session", KEY, isString);

    // Assert
    expect(actual).toBeUndefined();
  });

  it("значение не прошло isValid — undefined", () => {
    // Arrange
    writeStored("local", KEY, "строка");

    // Act
    const actual = readStored("local", KEY, isNumber);

    // Assert
    expect(actual).toBeUndefined();
  });

  it("строка, записанная без хелпера, читается как есть", () => {
    // Arrange
    localStorage.setItem(KEY, "dark");

    // Act
    const actual = readStored("local", KEY, isString);

    // Assert
    expect(actual).toBe("dark");
  });

  it("исключение storage не пробрасывается, а даёт undefined", () => {
    // Arrange
    const storage = brokenStorage();

    // Act
    const actual = readStored(storage, KEY, isString);

    // Assert
    expect(actual).toBeUndefined();
  });
});

describe("writeStored и removeStored", () => {
  it("исключение при записи даёт false", () => {
    // Arrange
    const storage = brokenStorage();

    // Act
    const written = writeStored(storage, KEY, "x");

    // Assert
    expect(written).toBe(false);
  });

  it("removeStored удаляет значение", () => {
    // Arrange
    writeStored("local", KEY, "x");

    // Act
    removeStored("local", KEY);

    // Assert
    expect(readStored("local", KEY, isString)).toBeUndefined();
  });

  it("исключение при удалении не пробрасывается", () => {
    // Arrange
    const storage = brokenStorage();

    // Act
    const act = () => removeStored(storage, KEY);

    // Assert
    expect(act).not.toThrow();
  });
});
