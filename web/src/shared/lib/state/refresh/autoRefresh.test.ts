import { afterEach, describe, expect, it, vi } from "vitest";
import { isAutoRefreshPaused, setAutoRefreshPaused, subscribeAutoRefresh } from "./autoRefresh";

afterEach(() => {
  setAutoRefreshPaused(false);
});

describe("флаг паузы автообновления", () => {
  it("установка паузы меняет значение и оповещает подписчика", () => {
    // Arrange
    const listener = vi.fn();
    subscribeAutoRefresh(listener);

    // Act
    setAutoRefreshPaused(true);

    // Assert
    expect(isAutoRefreshPaused()).toBe(true);
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("то же значение подписчиков не будит", () => {
    // Arrange
    const listener = vi.fn();
    setAutoRefreshPaused(true);
    subscribeAutoRefresh(listener);

    // Act
    setAutoRefreshPaused(true);

    // Assert
    expect(listener).not.toHaveBeenCalled();
  });

  it("отписанный слушатель не вызывается", () => {
    // Arrange
    const listener = vi.fn();
    const unsubscribe = subscribeAutoRefresh(listener);
    unsubscribe();

    // Act
    setAutoRefreshPaused(true);

    // Assert
    expect(listener).not.toHaveBeenCalled();
  });
});
