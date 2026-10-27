import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { BREAKPOINTS, useBreakpoint, useMediaQuery } from "~/shared/lib/core";
import { stubMatchMedia } from "~test/matchMedia";

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("useMediaQuery", () => {
  it("возвращает текущее значение запроса", () => {
    // Arrange
    stubMatchMedia(["(min-width: 100px)"]);

    // Act
    const { result } = renderHook(() => useMediaQuery("(min-width: 100px)"));

    // Assert
    expect(result.current).toBe(true);
  });

  it("обновляется при смене запроса и снимает подписку при размонтировании", () => {
    // Arrange
    const query = "(min-width: 100px)";
    const media = stubMatchMedia();
    const { result, unmount } = renderHook(() => useMediaQuery(query));

    // Act
    act(() => media.setMatches(query, true));

    // Assert
    expect(result.current).toBe(true);
    unmount();
    expect(media.listenerCount(query)).toBe(0);
  });
});

describe("useBreakpoint", () => {
  it("сверяет ширину с брейкпоинтом Tailwind", () => {
    // Arrange
    stubMatchMedia([`(min-width: ${BREAKPOINTS.xl}px)`]);

    // Act
    const { result } = renderHook(() => useBreakpoint("xl"));

    // Assert
    expect(result.current).toBe(true);
  });

  it("возвращает false, если окно уже брейкпоинта", () => {
    // Arrange
    stubMatchMedia();

    // Act
    const { result } = renderHook(() => useBreakpoint("2xl"));

    // Assert
    expect(result.current).toBe(false);
  });
});
