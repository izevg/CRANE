// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые данные (время, размеры, счётчики)
import { describe, expect, it } from "vitest";
import { textFormat } from "~/shared/lib/text";

const NBSP = " ";
const NOW = new Date(2026, 9, 21, 12, 0, 0).getTime();
const ru = textFormat("ru");
const en = textFormat("en");

const RELATIVE_RU = [
  [5000, "сейчас"],
  [12_000, "12 сек. назад"],
  [180_000, "3 мин. назад"],
  [7_200_000, "2 ч назад"],
  [30 * 3_600_000, "вчера"],
] as const;

const RELATIVE_EN = [
  [5000, "now"],
  [180_000, "3 min. ago"],
  [30 * 3_600_000, "yesterday"],
] as const;

describe("textFormat", () => {
  it("кэширует форматтеры на локаль", () => {
    // Arrange
    const locale = "ru";

    // Act
    const again = textFormat(locale);

    // Assert
    expect(again).toBe(ru);
  });

  it("number разделяет разряды по правилам локали", () => {
    // Arrange
    const value = 12_340;

    // Act
    const result = { ru: ru.number(value), en: en.number(value) };

    // Assert
    expect(result).toEqual({ ru: `12${NBSP}340`, en: "12,340" });
  });

  it("percent показывает один знак после запятой", () => {
    // Arrange
    const ratio = 0.942;

    // Act
    const result = { ru: ru.percent(ratio), en: en.percent(ratio) };

    // Assert
    expect(result).toEqual({ ru: `94,2${NBSP}%`, en: "94.2%" });
  });

  it("bytes переводит в терабайты и держит пробел неразрывным", () => {
    // Arrange
    const bytes = 18.4 * 1024 ** 4;

    // Act
    const result = ru.bytes(bytes);

    // Assert
    expect(result).toBe(`18,4${NBSP}ТБ`);
  });

  it("bytes показывает байты целым числом и не выходит за петабайты", () => {
    // Arrange
    const small = 512;
    const huge = 2048 * 1024 ** 5;

    // Act
    const result = { small: ru.bytes(small), huge: ru.bytes(huge) };

    // Assert
    expect(result.small).toBe(`512${NBSP}Б`);
    expect(result.huge).toContain("ПБ");
  });

  it("count с capped добавляет плюс, без capped — нет", () => {
    // Arrange
    const capped = { count: 1000, capped: true };
    const exact = { count: 1000, capped: false };

    // Act
    const result = { capped: ru.count(capped), exact: ru.count(exact) };

    // Assert
    expect(result).toEqual({ capped: `1${NBSP}000+`, exact: `1${NBSP}000` });
  });

  it.each(RELATIVE_RU)("relative (ru): сдвиг %i мс → «%s»", (shift, expected) => {
    // Arrange
    const ts = NOW - shift;

    // Act
    const result = ru.relative(ts, NOW);

    // Assert
    expect(result).toBe(expected);
  });

  it.each(RELATIVE_EN)("relative (en): сдвиг %i мс → «%s»", (shift, expected) => {
    // Arrange
    const ts = NOW - shift;

    // Act
    const result = en.relative(ts, NOW);

    // Assert
    expect(result).toBe(expected);
  });

  it("relative: старше двух суток показывает дату без года", () => {
    // Arrange
    const ts = new Date(2026, 9, 17, 11, 3).getTime();

    // Act
    const result = ru.relative(ts, NOW);

    // Assert
    expect(result).toBe("17 окт., 11:03");
  });

  it("relative: для другого года добавляет год", () => {
    // Arrange
    const ts = new Date(2025, 11, 31, 23, 59).getTime();

    // Act
    const result = ru.relative(ts, NOW);

    // Assert
    expect(result).toContain("2025");
  });

  it("relative: будущее время — «сейчас»", () => {
    // Arrange
    const ts = NOW + 60_000;

    // Act
    const result = ru.relative(ts, NOW);

    // Assert
    expect(result).toBe("сейчас");
  });

  it("relative: некорректная дата — прочерк", () => {
    // Arrange
    const ts = "not a date";

    // Act
    const result = ru.relative(ts, NOW);

    // Assert
    expect(result).toBe("—");
  });

  it("dateTime возвращает длинную дату со временем", () => {
    // Arrange
    const ts = new Date(2026, 9, 20, 14, 3, 7);

    // Act
    const result = ru.dateTime(ts);

    // Assert
    expect(result).toContain("20 октября 2026");
    expect(result).toContain("14:03:07");
  });
});
