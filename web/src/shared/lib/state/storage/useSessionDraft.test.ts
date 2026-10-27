import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { useSessionDraft } from "./useSessionDraft";

const FORM = "test";
const KEY = `crane.draft.${FORM}`;
const isText = (value: unknown): value is string => typeof value === "string";

afterEach(() => {
  sessionStorage.clear();
});

describe("useSessionDraft", () => {
  it("сохранённый черновик восстанавливается новым хуком после перезагрузки", () => {
    // Arrange
    const first = renderHook(() => useSessionDraft(FORM, isText));
    act(() => first.result.current[1]("Нужен доступ к релизу"));
    first.unmount();

    // Act
    const reloaded = renderHook(() => useSessionDraft(FORM, isText));

    // Assert
    expect(reloaded.result.current[0]).toBe("Нужен доступ к релизу");
  });

  it("невалидный черновик не восстанавливается", () => {
    // Arrange
    sessionStorage.setItem(KEY, JSON.stringify({ not: "текст" }));

    // Act
    const { result } = renderHook(() => useSessionDraft(FORM, isText));

    // Assert
    expect(result.current[0]).toBeUndefined();
  });

  it("clear удаляет черновик из хранилища и из состояния", () => {
    // Arrange
    const { result } = renderHook(() => useSessionDraft(FORM, isText));
    act(() => result.current[1]("черновик"));

    // Act
    act(() => result.current[2]());

    // Assert
    expect(result.current[0]).toBeUndefined();
    expect(sessionStorage.getItem(KEY)).toBeNull();
  });
});
