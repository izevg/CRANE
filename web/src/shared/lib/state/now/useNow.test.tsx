// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые данные (время, размеры, счётчики)
import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { MINUTE_MS, useNow } from "~/shared/lib/state";

const START = new Date(2026, 9, 27, 12, 0, 0);

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(START);
});

afterEach(() => {
  vi.useRealTimers();
});

describe("useNow", () => {
  it("два подписчика одного интервала делят один таймер", () => {
    // Arrange
    renderHook(() => useNow(MINUTE_MS));

    // Act
    renderHook(() => useNow(MINUTE_MS));

    // Assert
    expect(vi.getTimerCount()).toBe(1);
  });

  it("после отписки всех подписчиков таймер снимается", () => {
    // Arrange
    const first = renderHook(() => useNow(MINUTE_MS));
    const second = renderHook(() => useNow(MINUTE_MS));

    // Act
    act(() => {
      first.unmount();
      second.unmount();
    });

    // Assert
    expect(vi.getTimerCount()).toBe(0);
  });

  it("обновляет значение раз в интервал", async () => {
    // Arrange
    const { result } = renderHook(() => useNow(MINUTE_MS));
    const before = result.current;

    // Act
    await act(() => vi.advanceTimersByTimeAsync(MINUTE_MS));

    // Assert
    expect(result.current - before).toBe(MINUTE_MS);
  });

  it("разные интервалы идут на разных таймерах", () => {
    // Arrange
    renderHook(() => useNow(MINUTE_MS));

    // Act
    renderHook(() => useNow(MINUTE_MS * 2));

    // Assert
    expect(vi.getTimerCount()).toBe(2);
  });
});
