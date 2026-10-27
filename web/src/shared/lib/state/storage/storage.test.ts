import { afterEach, describe, expect, it, vi } from "vitest";
import { brokenStorage } from "~test/storage";
import { readStored, removeStored, writeStored } from "./storage";

const isMode = (value: string): value is "dark" | "light" => value === "dark" || value === "light";
const isAny = (value: string): value is string => value.length >= 0;
const KEY = "crane.test";

afterEach(() => {
  vi.restoreAllMocks();
  localStorage.clear();
  sessionStorage.clear();
});

describe("readStored", () => {
  it("читает записанную строку из выбранного хранилища", () => {
    // Arrange
    writeStored("session", KEY, "dark");

    // Act
    const actual = readStored("session", KEY, isMode);

    // Assert
    expect(actual).toBe("dark");
  });

  it("не путает local и session", () => {
    // Arrange
    writeStored("local", KEY, "dark");

    // Act
    const actual = readStored("session", KEY, isMode);

    // Assert
    expect(actual).toBeUndefined();
  });

  it("строка не прошла isValid — undefined", () => {
    // Arrange
    writeStored("local", KEY, "sepia");

    // Act
    const actual = readStored("local", KEY, isMode);

    // Assert
    expect(actual).toBeUndefined();
  });

  it("исключение storage не пробрасывается, а даёт undefined", () => {
    // Arrange
    writeStored("local", KEY, "dark");
    brokenStorage();

    // Act
    const actual = readStored("local", KEY, isAny);

    // Assert
    expect(actual).toBeUndefined();
  });
});

describe("writeStored и removeStored", () => {
  it("строка кладётся в хранилище без кодирования", () => {
    // Arrange
    // Act
    writeStored("local", KEY, "dark");

    // Assert
    expect(localStorage.getItem(KEY)).toBe("dark");
  });

  it("исключение при записи даёт false", () => {
    // Arrange
    brokenStorage();

    // Act
    const written = writeStored("local", KEY, "x");

    // Assert
    expect(written).toBe(false);
  });

  it("removeStored удаляет значение", () => {
    // Arrange
    writeStored("local", KEY, "x");

    // Act
    removeStored("local", KEY);

    // Assert
    expect(readStored("local", KEY, isAny)).toBeUndefined();
  });

  it("исключение при удалении не пробрасывается", () => {
    // Arrange
    brokenStorage();

    // Act
    const act = () => removeStored("local", KEY);

    // Assert
    expect(act).not.toThrow();
  });
});
