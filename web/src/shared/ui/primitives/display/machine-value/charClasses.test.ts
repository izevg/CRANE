// biome-ignore-all lint/style/noMagicNumbers: граница «≤ 64 знаков» из требования записана литералами намеренно, без импорта константы
import { describe, expect, it } from "vitest";
import { classOf, splitSeries } from "./charClasses";

describe("splitSeries", () => {
  it("«a1_B-9» делится на серии букв, цифр и знаков", () => {
    // Arrange
    const value = "a1_B-9";

    // Act
    const actual = splitSeries(value);

    // Assert
    expect(actual?.map(({ kind, text }) => [kind, text])).toEqual([
      ["letter", "a"],
      ["digit", "1"],
      ["symbol", "_"],
      ["letter", "B"],
      ["symbol", "-"],
      ["digit", "9"],
    ]);
  });

  it("подряд идущие символы одного класса сливаются в одну серию", () => {
    // Arrange
    const value = "ab12__cd";

    // Act
    const actual = splitSeries(value);

    // Assert
    expect(actual?.map(({ text }) => text)).toEqual(["ab", "12", "__", "cd"]);
  });

  it("серии склеиваются обратно в точное исходное значение", () => {
    // Arrange
    const value = "crp_a1b2c3d4+/=";

    // Act
    const actual = splitSeries(value)
      ?.map(({ text }) => text)
      .join("");

    // Assert
    expect(actual).toBe(value);
  });

  it("знак усечения «…» — символ", () => {
    // Arrange
    const value = "a88d…6973";

    // Act
    const actual = splitSeries(value);

    // Assert
    expect(actual?.map(({ kind }) => kind)).toEqual([
      "letter",
      "digit",
      "letter",
      "symbol",
      "digit",
    ]);
  });

  it("значение длиннее 64 знаков не размечается", () => {
    // Arrange
    const value = "a".repeat(65);

    // Act
    const actual = splitSeries(value);

    // Assert
    expect(actual).toBeNull();
  });

  it("значение ровно в 64 знака размечается", () => {
    // Arrange
    const value = "1".repeat(64);

    // Act
    const actual = splitSeries(value);

    // Assert
    expect(actual).toEqual([{ kind: "digit", text: value, start: 0 }]);
  });

  it("пустая строка даёт пустой список серий", () => {
    // Arrange
    const value = "";

    // Act
    const actual = splitSeries(value);

    // Assert
    expect(actual).toEqual([]);
  });
});

describe("classOf", () => {
  it("пробел, перенос строки и кириллица остаются цветом текста", () => {
    // Arrange
    const chars = [" ", "\n", "ж"];

    // Act
    const actual = chars.map(classOf);

    // Assert
    expect(actual).toEqual(["letter", "letter", "letter"]);
  });
});
