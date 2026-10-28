import { describe, expect, it } from "vitest";
import { ApiProblem } from "~/shared/api";
import { retryDelay, shouldRetry } from "./retry";

const UNRETRYABLE = ["unauthenticated", "step-up", "csrf", "planned"] as const;
const RETRY_AFTER_MS = 7000;
const FAR_RETRY = 20;
const MAX_DELAY_MS = 30_000;
const CLIENT_ERRORS = {
  badRequest: 400,
  forbidden: 403,
  notFound: 404,
  conflict: 409,
  precondition: 412,
  unprocessable: 422,
};
const SERVER_ERRORS = { internal: 500, badGateway: 502, unavailable: 503 };
const DELAYS_MS = { first: 1000, second: 2000, third: 4000 };

const STATUS = {
  notFound: 404,
  timeout: 408,
  tooManyRequests: 429,
  unavailable: 503,
} as const;

function problem(status: number, retryAfterMs?: number): ApiProblem {
  return new ApiProblem({
    kind: "problem",
    status,
    ...(retryAfterMs === undefined ? {} : { retryAfterMs }),
  });
}

describe("shouldRetry: что повторяем", () => {
  it.each(Object.values(CLIENT_ERRORS))("4xx (%i) не повторяется", (status) => {
    // Arrange
    const error = problem(status);

    // Act
    const retry = shouldRetry(0, error);

    // Assert
    expect(retry).toBe(false);
  });

  it("408 повторяется", () => {
    // Arrange
    const error = problem(STATUS.timeout);

    // Act
    const retry = shouldRetry(0, error);

    // Assert
    expect(retry).toBe(true);
  });

  it("429 с Retry-After повторяется", () => {
    // Arrange
    const error = problem(STATUS.tooManyRequests, RETRY_AFTER_MS);

    // Act
    const retry = shouldRetry(0, error);

    // Assert
    expect(retry).toBe(true);
  });

  it("Retry-After у 404 ничего не меняет: 4xx не повторяется", () => {
    // Arrange
    const error = problem(STATUS.notFound, RETRY_AFTER_MS);

    // Act
    const retry = shouldRetry(0, error);

    // Assert
    expect(retry).toBe(false);
  });

  it("429 без Retry-After не повторяется: срок ожидания неизвестен", () => {
    // Arrange
    const error = problem(STATUS.tooManyRequests);

    // Act
    const retry = shouldRetry(0, error);

    // Assert
    expect(retry).toBe(false);
  });

  it.each(Object.values(SERVER_ERRORS))("%i повторяется", (status) => {
    // Arrange
    const error = problem(status);

    // Act
    const retry = shouldRetry(0, error);

    // Assert
    expect(retry).toBe(true);
  });

  it("сбой сети повторяется", () => {
    // Arrange
    const error = new ApiProblem({ kind: "network", status: 0 });

    // Act
    const retry = shouldRetry(1, error);

    // Assert
    expect(retry).toBe(true);
  });

  it("не больше двух повторов", () => {
    // Arrange
    const error = problem(STATUS.unavailable);

    // Act
    const retry = shouldRetry(2, error);

    // Assert
    expect(retry).toBe(false);
  });

  it.each(UNRETRYABLE)("%s повтором не лечится", (kind) => {
    // Arrange
    const error = new ApiProblem({ kind, status: 0 });

    // Act
    const retry = shouldRetry(0, error);

    // Assert
    expect(retry).toBe(false);
  });

  it("ошибка не из API (баг в коде) не повторяется", () => {
    // Arrange
    const error = new TypeError("x is undefined");

    // Act
    const retry = shouldRetry(0, error);

    // Assert
    expect(retry).toBe(false);
  });
});

describe("retryDelay", () => {
  it("Retry-After сервера важнее собственной паузы", () => {
    // Arrange
    const error = problem(STATUS.tooManyRequests, RETRY_AFTER_MS);

    // Act
    const delay = retryDelay(0, error);

    // Assert
    expect(delay).toBe(RETRY_AFTER_MS);
  });

  it("без Retry-After пауза растёт экспоненциально", () => {
    // Arrange
    const error = problem(STATUS.unavailable);

    // Act
    const delays = [0, 1, 2].map((count) => retryDelay(count, error));

    // Assert
    expect(delays).toEqual([DELAYS_MS.first, DELAYS_MS.second, DELAYS_MS.third]);
  });

  it("пауза не превышает потолок", () => {
    // Arrange
    const error = problem(STATUS.unavailable);

    // Act
    const delay = retryDelay(FAR_RETRY, error);

    // Assert
    expect(delay).toBe(MAX_DELAY_MS);
  });
});
