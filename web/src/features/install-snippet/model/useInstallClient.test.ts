import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { useInstallClient } from "./useInstallClient";

const KEY = "crane.install.npm";

afterEach(() => {
  localStorage.clear();
});

describe("useInstallClient", () => {
  it("без предпочтения — undefined: сервер выберет клиента сам", () => {
    // Arrange
    // Act
    const { result } = renderHook(() => useInstallClient("npm"));

    // Assert
    expect(result.current[0]).toBeUndefined();
  });

  it("сохранённое предпочтение возвращается для первого запроса", () => {
    // Arrange
    localStorage.setItem(KEY, "pnpm");

    // Act
    const { result } = renderHook(() => useInstallClient("npm"));

    // Assert
    expect(result.current[0]).toBe("pnpm");
  });

  it("мусор в хранилище не считается предпочтением", () => {
    // Arrange
    localStorage.setItem(KEY, "bogus");

    // Act
    const { result } = renderHook(() => useInstallClient("npm"));

    // Assert
    expect(result.current[0]).toBeUndefined();
  });

  it("явный выбор возвращается хуком сразу и запоминается в хранилище", () => {
    // Arrange
    const { result } = renderHook(() => useInstallClient("npm"));

    // Act
    act(() => result.current[1]("pnpm"));

    // Assert
    expect(localStorage.getItem(KEY)).toBe("pnpm");
  });
});
