import { describe, expect, it } from "vitest";
import { ApiProblem } from "../http/problem";
import { polled } from "./polled";

const STATUS = {
  unauthorized: 401,
} as const;

const POLL_MS = 30_000;
const CUSTOM_MS = 5000;

type Interval = (query: { state: { error: unknown } }) => number | false;
const HEALTHY = { state: { error: null } };

function intervalOf(options: { refetchInterval?: unknown }): Interval {
  const { refetchInterval } = options;
  if (typeof refetchInterval !== "function") {
    throw new Error("refetchInterval не функция");
  }
  return refetchInterval as Interval;
}

describe("polled", () => {
  it("операция с каденсом получает его в миллисекундах", () => {
    // Arrange
    const options = {};

    // Act
    const result = polled(options, "getStatus", () => false);

    // Assert
    expect(intervalOf(result)(HEALTHY)).toBe(POLL_MS);
  });

  it("пауза останавливает опрос", () => {
    // Arrange
    const result = polled({}, "getStatus", () => true);

    // Act
    const interval = intervalOf(result)(HEALTHY);

    // Assert
    expect(interval).toBe(false);
  });

  it("после снятия паузы опрос возобновляется без пересоздания опций", () => {
    // Arrange
    let paused = true;
    const result = polled({}, "getStatus", () => paused);
    const whilePaused = intervalOf(result)(HEALTHY);
    paused = false;

    // Act
    const resumed = intervalOf(result)(HEALTHY);

    // Assert
    expect(whilePaused).toBe(false);
    expect(resumed).toBe(POLL_MS);
  });

  it("после 401 опрос останавливается: повторы только множат ошибки", () => {
    // Arrange
    const result = polled({}, "getStatus", () => false);
    const failed = {
      state: { error: new ApiProblem({ kind: "unauthenticated", status: STATUS.unauthorized }) },
    };

    // Act
    const interval = intervalOf(result)(failed);

    // Assert
    expect(interval).toBe(false);
  });

  it("временный сбой сети опрос не останавливает", () => {
    // Arrange
    const result = polled({}, "getStatus", () => false);
    const failed = { state: { error: new ApiProblem({ kind: "network", status: 0 }) } };

    // Act
    const interval = intervalOf(result)(failed);

    // Assert
    expect(interval).toBe(POLL_MS);
  });

  it("операция без каденса не опрашивается", () => {
    // Arrange
    const options = { staleTime: 1, refetchInterval: undefined };

    // Act
    const result = polled(options, "getMe", () => false);

    // Assert
    expect(result).toBe(options);
    expect(result.refetchInterval).toBeUndefined();
  });

  it("каденс, заданный вызывающим, не перезаписывается", () => {
    // Arrange
    const options = { refetchInterval: CUSTOM_MS };

    // Act
    const result = polled(options, "getStatus", () => false);

    // Assert
    expect(result.refetchInterval).toBe(CUSTOM_MS);
  });
});
