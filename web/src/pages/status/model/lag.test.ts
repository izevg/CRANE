import { describe, expect, it } from "vitest";
import { lagOf } from "./lag";

const SECONDS = { few: 12, justUnder: 59, oneMinute: 60, nine: 540, rounded: 95 };

describe("lagOf", () => {
  it.each([null, undefined])("нет данных (%s) — null, а не 0", (seconds) => {
    // Arrange
    const input = seconds;

    // Act
    const lag = lagOf(input);

    // Assert
    expect(lag).toBeNull();
  });

  it("меньше минуты — секунды", () => {
    // Arrange
    const seconds = SECONDS.justUnder;

    // Act
    const lag = lagOf(seconds);

    // Assert
    expect(lag).toEqual({ value: SECONDS.justUnder, unit: "s" });
  });

  it("ноль секунд — это данные «нет отставания», а не их отсутствие", () => {
    // Arrange
    const seconds = 0;

    // Act
    const lag = lagOf(seconds);

    // Assert
    expect(lag).toEqual({ value: 0, unit: "s" });
  });

  it("ровно минута — уже минуты", () => {
    // Arrange
    const seconds = SECONDS.oneMinute;

    // Act
    const lag = lagOf(seconds);

    // Assert
    expect(lag).toEqual({ value: 1, unit: "min" });
  });

  it("минуты округляются до целых", () => {
    // Arrange
    const seconds = SECONDS.rounded;

    // Act
    const lag = lagOf(seconds);

    // Assert
    expect(lag).toEqual({ value: 2, unit: "min" });
  });
});
