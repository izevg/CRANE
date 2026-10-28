import { describe, expect, it } from "vitest";
import { type ProbeStatus, summarize } from "./summary";

const checks = (...statuses: ProbeStatus[]) => statuses.map((status) => ({ status }));

describe("summarize", () => {
  it("считает проверки по статусам", () => {
    // Arrange
    const list = checks("pass", "pass", "warn", "fail", "fail", "fail");

    // Act
    const actual = summarize(list);

    // Assert
    expect(actual).toEqual({ pass: 2, warn: 1, fail: 3 });
  });

  it("пустой список даёт нули", () => {
    // Arrange
    const list = checks();

    // Act
    const actual = summarize(list);

    // Assert
    expect(actual).toEqual({ pass: 0, warn: 0, fail: 0 });
  });

  it("пропущенные проверки не попадают ни в одну графу", () => {
    // Arrange
    const list = checks("skipped", "pass", "skipped");

    // Act
    const actual = summarize(list);

    // Assert
    expect(actual).toEqual({ pass: 1, warn: 0, fail: 0 });
  });

  it("предупреждение и ошибка считаются раздельно", () => {
    // Arrange
    const list = checks("warn", "warn", "fail");

    // Act
    const actual = summarize(list);

    // Assert
    expect(actual).toEqual({ pass: 0, warn: 2, fail: 1 });
  });
});
