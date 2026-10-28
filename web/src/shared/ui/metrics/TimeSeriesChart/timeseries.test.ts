import { describe, expect, it } from "vitest";
import { textFormat } from "~/shared/lib/text";
import {
  formatValueWithUnit,
  lastIndexOf,
  makeTickFormatter,
  mergeSeries,
  type Row,
  type Series,
} from "./timeseries";

const HOUR_MS = 3_600_000;
const DAY_MS = 24 * HOUR_MS;
const START = Date.parse("2026-01-03T12:00:00Z");
const AMOUNT = 1234.5;
const CLOCK_ONLY = /^\d{2}:\d{2}$/;
const WITH_DATE = /^\d{1,2} [A-Za-z]{3},? \d{2}:\d{2}$/;

const series = (id: string, points: [number, number][]): Series => ({
  id,
  label: id,
  points: points.map(([t, v]) => ({ t, v })),
});

describe("mergeSeries", () => {
  it("объединяет ряды по времени и сортирует строки по возрастанию", () => {
    // Arrange
    const input = [
      series("a", [
        [10, 10],
        [1, 1],
      ]),
      series("b", [[2, 2]]),
    ];

    // Act
    const rows = mergeSeries(input);

    // Assert
    expect(rows).toEqual([
      { t: 1, a: 1 },
      { t: 2, b: 2 },
      { t: 10, a: 10 },
    ]);
  });

  it("совпавшее время даёт одну строку со значениями обеих серий", () => {
    // Arrange
    const input = [series("a", [[1, 1]]), series("b", [[1, 2]])];

    // Act
    const rows = mergeSeries(input);

    // Assert
    expect(rows).toEqual([{ t: 1, a: 1, b: 2 }]);
  });

  it("без рядов строк нет", () => {
    // Arrange
    const input: Series[] = [];

    // Act
    const rows = mergeSeries(input);

    // Assert
    expect(rows).toEqual([]);
  });
});

describe("makeTickFormatter", () => {
  it("окно ровно в сутки показывает только время", () => {
    // Arrange
    const rows: Row[] = [{ t: START }, { t: START + DAY_MS }];

    // Act
    const label = makeTickFormatter(rows, "en-GB")(START);

    // Assert
    expect(label).toMatch(CLOCK_ONLY);
  });

  it("окно шире суток добавляет дату", () => {
    // Arrange
    const rows: Row[] = [{ t: START }, { t: START + DAY_MS + 1 }];

    // Act
    const label = makeTickFormatter(rows, "en-GB")(START);

    // Assert
    expect(label).toMatch(WITH_DATE);
  });

  it("без строк формат только по времени, без ошибки", () => {
    // Arrange
    const rows: Row[] = [];

    // Act
    const label = makeTickFormatter(rows, "en-GB")(START);

    // Assert
    expect(label).toMatch(CLOCK_ONLY);
  });
});

describe("formatValueWithUnit", () => {
  const cases: [string, string | undefined, string][] = [
    ["с единицей", "ms", "1,234.5 ms"],
    ["без единицы", undefined, "1,234.5"],
    ["пустая единица не печатается", "", "1,234.5"],
  ];

  it.each(cases)("%s", (_name, unit, expected) => {
    // Arrange
    const format = textFormat("en");

    // Act
    const text = formatValueWithUnit(AMOUNT, unit, format);

    // Assert
    expect(text).toBe(expected);
  });
});

describe("lastIndexOf", () => {
  const rows: Row[] = [
    { t: 1, a: 1 },
    { t: 2, b: 2 },
    { t: 10, a: 10 },
    { t: 11, b: 11 },
  ];

  it("возвращает индекс последней строки серии", () => {
    // Arrange
    const id = "a";

    // Act
    const index = lastIndexOf(rows, id);

    // Assert
    expect(index).toBe(2);
  });

  it("нет значений серии — минус один", () => {
    // Arrange
    const id = "missing";

    // Act
    const index = lastIndexOf(rows, id);

    // Assert
    expect(index).toBe(-1);
  });
});
