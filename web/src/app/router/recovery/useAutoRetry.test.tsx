import { renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createBackoff } from "./backoff";
import { useAutoRetry } from "./useAutoRetry";

const FIRST_DELAY_MS = 5000;
const SECOND_DELAY_MS = 10_000;
const LONG_WAIT_MS = 100_000;
const RETRY_AFTER_MS = 20_000;

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useAutoRetry", () => {
  it("повторяет через 5 с", async () => {
    // Arrange
    const retry = vi.fn();
    renderHook(() => useAutoRetry(true, retry, undefined, createBackoff()));

    // Act
    await vi.advanceTimersByTimeAsync(FIRST_DELAY_MS);

    // Assert
    expect(retry).toHaveBeenCalledTimes(1);
  });

  it("раньше срока не повторяет", async () => {
    // Arrange
    const retry = vi.fn();
    renderHook(() => useAutoRetry(true, retry, undefined, createBackoff()));

    // Act
    await vi.advanceTimersByTimeAsync(FIRST_DELAY_MS - 1);

    // Assert
    expect(retry).not.toHaveBeenCalled();
  });

  it("событие online повторяет сразу", () => {
    // Arrange
    const retry = vi.fn();
    renderHook(() => useAutoRetry(true, retry, undefined, createBackoff()));

    // Act
    window.dispatchEvent(new Event("online"));

    // Assert
    expect(retry).toHaveBeenCalledTimes(1);
  });

  it("после online расписание начинается сначала", async () => {
    // Arrange
    const backoff = createBackoff();
    const { rerender } = renderHook(({ id }) => useAutoRetry(true, id, undefined, backoff), {
      initialProps: { id: vi.fn() },
    });
    window.dispatchEvent(new Event("online"));
    const retry = vi.fn();
    rerender({ id: retry });

    // Act
    await vi.advanceTimersByTimeAsync(FIRST_DELAY_MS);

    // Assert
    expect(retry).toHaveBeenCalledTimes(1);
  });

  it("следующий экран ошибки ждёт дольше: 10 с", async () => {
    // Arrange
    const backoff = createBackoff();
    const first = renderHook(() => useAutoRetry(true, vi.fn(), undefined, backoff));
    await vi.advanceTimersByTimeAsync(FIRST_DELAY_MS);
    first.unmount();
    const retry = vi.fn();
    renderHook(() => useAutoRetry(true, retry, undefined, backoff));

    // Act
    await vi.advanceTimersByTimeAsync(SECOND_DELAY_MS - 1);

    // Assert
    expect(retry).not.toHaveBeenCalled();
  });

  it("Retry-After сервера удлиняет паузу", async () => {
    // Arrange
    const retry = vi.fn();
    renderHook(() => useAutoRetry(true, retry, RETRY_AFTER_MS, createBackoff()));

    // Act
    await vi.advanceTimersByTimeAsync(FIRST_DELAY_MS);

    // Assert
    expect(retry).not.toHaveBeenCalled();
  });

  it("неактивный автоповтор ничего не планирует и не слушает", async () => {
    // Arrange
    const retry = vi.fn();
    renderHook(() => useAutoRetry(false, retry, undefined, createBackoff()));

    window.dispatchEvent(new Event("online"));

    // Act
    await vi.advanceTimersByTimeAsync(LONG_WAIT_MS);

    // Assert
    expect(retry).not.toHaveBeenCalled();
  });

  it("при размонтировании таймер и слушатель сняты: утечек нет", async () => {
    // Arrange
    const retry = vi.fn();
    const { unmount } = renderHook(() => useAutoRetry(true, retry, undefined, createBackoff()));
    unmount();
    window.dispatchEvent(new Event("online"));

    // Act
    await vi.advanceTimersByTimeAsync(LONG_WAIT_MS);

    // Assert
    expect(retry).not.toHaveBeenCalled();
    expect(vi.getTimerCount()).toBe(0);
  });
});
