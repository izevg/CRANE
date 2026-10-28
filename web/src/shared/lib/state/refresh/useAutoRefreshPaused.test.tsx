import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { setAutoRefreshPaused } from "./autoRefresh";
import { useAutoRefreshPaused } from "./useAutoRefreshPaused";

afterEach(() => {
  setAutoRefreshPaused(false);
});

describe("useAutoRefreshPaused", () => {
  it("перерисовывает компонент при переключении паузы", () => {
    // Arrange
    const { result } = renderHook(() => useAutoRefreshPaused());

    // Act
    act(() => setAutoRefreshPaused(true));

    // Assert
    expect(result.current).toBe(true);
  });
});
