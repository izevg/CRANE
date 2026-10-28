import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createFlag } from "./flag";

describe("createFlag", () => {
  it("начальное значение задаёт вызывающий", () => {
    // Arrange
    const flag = createFlag(true);

    // Act
    const value = flag.get();

    // Assert
    expect(value).toBe(true);
  });

  it("установка меняет значение и оповещает подписчика", () => {
    // Arrange
    const flag = createFlag();
    const listener = vi.fn();
    flag.subscribe(listener);

    // Act
    flag.set(true);

    // Assert
    expect(flag.get()).toBe(true);
    expect(listener).toHaveBeenCalledTimes(1);
  });

  it("то же значение подписчиков не будит", () => {
    // Arrange
    const flag = createFlag(true);
    const listener = vi.fn();
    flag.subscribe(listener);

    // Act
    flag.set(true);

    // Assert
    expect(listener).not.toHaveBeenCalled();
  });

  it("отписанный слушатель не вызывается", () => {
    // Arrange
    const flag = createFlag();
    const listener = vi.fn();
    const unsubscribe = flag.subscribe(listener);
    unsubscribe();

    // Act
    flag.set(true);

    // Assert
    expect(listener).not.toHaveBeenCalled();
  });

  it("флаги независимы друг от друга", () => {
    // Arrange
    const first = createFlag();
    const second = createFlag();

    // Act
    first.set(true);

    // Assert
    expect(second.get()).toBe(false);
  });

  it("use перерисовывает компонент при изменении флага", () => {
    // Arrange
    const flag = createFlag();
    const { result } = renderHook(() => flag.use());

    // Act
    act(() => flag.set(true));

    // Assert
    expect(result.current).toBe(true);
  });
});
