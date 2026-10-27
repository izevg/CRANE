import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { initColorScheme, useColorMode } from "~/shared/lib/state";
import { stubMatchMedia } from "~test/matchMedia";
import { COLOR_SCHEME_STORAGE_KEY, readStoredMode, writeStoredMode } from "./colorScheme";

function brokenStorage(): Storage {
  const fail = (): never => {
    throw new Error("storage недоступен");
  };
  return { getItem: fail, setItem: fail, removeItem: fail, clear: fail, key: fail, length: 0 };
}

beforeEach(() => {
  localStorage.clear();
});

afterEach(() => {
  vi.unstubAllGlobals();
  document.documentElement.removeAttribute("data-theme");
});

describe("readStoredMode", () => {
  it("с недоступным storage возвращает system без исключения", () => {
    // Arrange
    const storage = brokenStorage();

    // Act
    const mode = readStoredMode(storage);

    // Assert
    expect(mode).toBe("system");
  });

  it("с неизвестным значением возвращает system", () => {
    // Arrange
    localStorage.setItem(COLOR_SCHEME_STORAGE_KEY, "sepia");

    // Act
    const mode = readStoredMode();

    // Assert
    expect(mode).toBe("system");
  });

  it("читает сохранённый режим", () => {
    // Arrange
    localStorage.setItem(COLOR_SCHEME_STORAGE_KEY, "dark");

    // Act
    const mode = readStoredMode();

    // Assert
    expect(mode).toBe("dark");
  });
});

describe("writeStoredMode", () => {
  it("с недоступным storage возвращает false без исключения", () => {
    // Arrange
    const storage = brokenStorage();

    // Act
    const written = writeStoredMode("dark", storage);

    // Assert
    expect(written).toBe(false);
  });
});

describe("useColorMode", () => {
  it("смена режима применяет data-theme к <html> и сохраняет выбор", () => {
    // Arrange
    stubMatchMedia();
    const { result } = renderHook(() => useColorMode());

    // Act
    act(() => result.current.setMode("dark"));

    // Assert
    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
    expect(result.current.resolved).toBe("dark");
    expect(localStorage.getItem(COLOR_SCHEME_STORAGE_KEY)).toBe("dark");
  });

  it("режим system следует за prefers-color-scheme", () => {
    // Arrange
    stubMatchMedia(["(prefers-color-scheme: dark)"]);
    const { result } = renderHook(() => useColorMode());

    // Act
    act(() => result.current.setMode("system"));

    // Assert
    expect(result.current.resolved).toBe("dark");
  });
});

describe("initColorScheme", () => {
  it("применяет сохранённый режим к <html> при старте", () => {
    // Arrange
    stubMatchMedia();
    localStorage.setItem(COLOR_SCHEME_STORAGE_KEY, "dark");

    // Act
    const cleanup = initColorScheme();

    // Assert
    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
    cleanup();
  });
});
