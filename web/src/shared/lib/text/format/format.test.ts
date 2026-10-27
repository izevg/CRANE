// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые данные (время, размеры, счётчики)
import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatCount,
  formatDateTime,
  formatNumber,
  formatPercent,
  formatRelative,
  pluralRu,
} from "~/shared/lib/text";

const NBSP = " ";
const NOW = new Date(2026, 9, 21, 12, 0, 0).getTime();
const VERSIONS = ["версия", "версии", "версий"] as const;

const RELATIVE_CASES = [
  [5000, "только что"],
  [12_000, "12 с назад"],
  [180_000, "3 мин назад"],
  [7_200_000, "2 ч назад"],
] as const;

const PLURAL_CASES = [
  [1, "версия"],
  [2, "версии"],
  [5, "версий"],
  [11, "версий"],
  [21, "версия"],
] as const;

describe("formatNumber", () => {
  it("разделяет разряды неразрывным пробелом", () => {
    // Arrange
    const value = 12_340;

    // Act
    const result = formatNumber(value);

    // Assert
    expect(result).toBe(`12${NBSP}340`);
  });
});

describe("formatPercent", () => {
  it("форматирует долю с запятой и неразрывным пробелом перед знаком", () => {
    // Arrange
    const ratio = 0.942;

    // Act
    const result = formatPercent(ratio);

    // Assert
    expect(result).toBe(`94,2${NBSP}%`);
  });
});

describe("formatBytes", () => {
  it("переводит в терабайты с одним знаком", () => {
    // Arrange
    const bytes = 18.4 * 1024 ** 4;

    // Act
    const result = formatBytes(bytes);

    // Assert
    expect(result).toBe(`18,4${NBSP}ТБ`);
  });

  it("байты показывает целым числом", () => {
    // Arrange
    const bytes = 512;

    // Act
    const result = formatBytes(bytes);

    // Assert
    expect(result).toBe(`512${NBSP}Б`);
  });

  it("не выходит за петабайты", () => {
    // Arrange
    const bytes = 2048 * 1024 ** 5;

    // Act
    const result = formatBytes(bytes);

    // Assert
    expect(result).toContain("ПБ");
  });
});

describe("formatCount", () => {
  it("усечённый счётчик получает плюс", () => {
    // Arrange
    const value = { count: 1000, capped: true };

    // Act
    const result = formatCount(value);

    // Assert
    expect(result).toBe(`1${NBSP}000+`);
  });

  it("точный счётчик без плюса", () => {
    // Arrange
    const value = { count: 1000, capped: false };

    // Act
    const result = formatCount(value);

    // Assert
    expect(result).toBe(`1${NBSP}000`);
  });
});

describe("pluralRu", () => {
  it.each(PLURAL_CASES)("для %i выбирает «%s»", (count, expected) => {
    // Arrange
    const forms = VERSIONS;

    // Act
    const result = pluralRu(count, forms);

    // Assert
    expect(result).toBe(expected);
  });
});

describe("formatRelative", () => {
  it.each(RELATIVE_CASES)("для сдвига %i мс возвращает «%s»", (shift, expected) => {
    // Arrange
    const ts = NOW - shift;

    // Act
    const result = formatRelative(ts, NOW);

    // Assert
    expect(result).toBe(expected);
  });

  it("старше суток показывает дату без года", () => {
    // Arrange
    const ts = new Date(2026, 9, 20, 11, 3).getTime();

    // Act
    const result = formatRelative(ts, NOW);

    // Assert
    expect(result).toBe("20 окт., 11:03");
  });

  it("для другого года добавляет год", () => {
    // Arrange
    const ts = new Date(2025, 11, 31, 23, 59).getTime();

    // Act
    const result = formatRelative(ts, NOW);

    // Assert
    expect(result).toContain("2025");
  });

  it("будущее время показывает как «только что»", () => {
    // Arrange
    const ts = NOW + 60_000;

    // Act
    const result = formatRelative(ts, NOW);

    // Assert
    expect(result).toBe("только что");
  });

  it("некорректная дата даёт прочерк", () => {
    // Arrange
    const ts = "not a date";

    // Act
    const result = formatRelative(ts, NOW);

    // Assert
    expect(result).toBe("—");
  });
});

describe("formatDateTime", () => {
  it("возвращает длинную дату со временем", () => {
    // Arrange
    const ts = new Date(2026, 9, 20, 14, 3, 7);

    // Act
    const result = formatDateTime(ts);

    // Assert
    expect(result).toContain("20 октября 2026");
    expect(result).toContain("14:03:07");
  });
});
