import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { useInstallClient } from "./useInstallClient";

const KEY = "crane.install.npm";

afterEach(() => {
  localStorage.clear();
});

describe("useInstallClient", () => {
  it("до ответа сервера без сохранённого клиента — undefined: сервер выберет сам", () => {
    // Arrange
    // Act
    const { result } = renderHook(() => useInstallClient("npm"));

    // Assert
    expect(result.current[0]).toBeUndefined();
  });

  it("до ответа сервера возвращает сохранённого клиента для первого запроса", () => {
    // Arrange
    localStorage.setItem(KEY, "pnpm");

    // Act
    const { result } = renderHook(() => useInstallClient("npm"));

    // Assert
    expect(result.current[0]).toBe("pnpm");
  });

  it("мусор в хранилище не считается клиентом", () => {
    // Arrange
    localStorage.setItem(KEY, "bogus");

    // Act
    const { result } = renderHook(() => useInstallClient("npm"));

    // Assert
    expect(result.current[0]).toBeUndefined();
  });

  it("после ответа сохранённый клиент вне available_clients заменяется первым доступным", () => {
    // Arrange
    localStorage.setItem(KEY, "poetry");

    // Act
    const { result } = renderHook(() => useInstallClient("npm", ["npm", "pnpm"]));

    // Assert
    expect(result.current[0]).toBe("npm");
  });

  it("после ответа сохранённый доступный клиент остаётся", () => {
    // Arrange
    localStorage.setItem(KEY, "pnpm");

    // Act
    const { result } = renderHook(() => useInstallClient("npm", ["npm", "pnpm"]));

    // Assert
    expect(result.current[0]).toBe("pnpm");
  });

  it("явный выбор запоминается в хранилище", () => {
    // Arrange
    const { result } = renderHook(() => useInstallClient("npm", ["npm", "pnpm"]));

    // Act
    act(() => result.current[1]("pnpm"));

    // Assert
    expect(localStorage.getItem(KEY)).toBe("pnpm");
  });
});
