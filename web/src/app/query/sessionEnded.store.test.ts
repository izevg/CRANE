import { describe, expect, it, vi } from "vitest";
import { isSessionEnded, markSessionEnded, subscribeSessionEnded } from "./sessionEnded";

describe("флаг завершённой сессии", () => {
  it("первая отметка оповещает подписчика, повторная — нет", () => {
    // Arrange
    const listener = vi.fn();
    subscribeSessionEnded(listener);

    markSessionEnded();

    // Act
    markSessionEnded();

    // Assert
    expect(listener).toHaveBeenCalledTimes(1);
    expect(isSessionEnded()).toBe(true);
  });

  it("отписанный слушатель больше не вызывается", () => {
    // Arrange
    const listener = vi.fn();
    const unsubscribe = subscribeSessionEnded(listener);
    unsubscribe();

    // Act
    markSessionEnded();

    // Assert
    expect(listener).not.toHaveBeenCalled();
  });
});
