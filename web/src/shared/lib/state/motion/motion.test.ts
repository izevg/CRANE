// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые данные (время, размеры, счётчики)
import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import {
  durBase,
  durFast,
  durInstant,
  durMax,
  durSlow,
  toSeconds,
  usePrefersReducedMotion,
} from "~/shared/lib/state";
import motionCss from "~/shared/styles/motion.css?raw";
import { stubMatchMedia } from "~test/matchMedia";

afterEach(() => {
  vi.unstubAllGlobals();
});

function cssDuration(name: string): number {
  const match = new RegExp(`--dur-${name}:\\s*(\\d+)ms`).exec(motionCss);
  return Number(match?.[1]);
}

describe("токены длительности", () => {
  it("совпадают со значениями motion.css", () => {
    // Arrange
    const expected = {
      instant: durInstant,
      fast: durFast,
      base: durBase,
      slow: durSlow,
      max: durMax,
    };

    // Act
    const fromCss = {
      instant: cssDuration("instant"),
      fast: cssDuration("fast"),
      base: cssDuration("base"),
      slow: cssDuration("slow"),
      max: cssDuration("max"),
    };

    // Assert
    expect(fromCss).toEqual(expected);
  });

  it("toSeconds переводит миллисекунды в секунды для Motion", () => {
    // Arrange
    const ms = durBase;

    // Act
    const seconds = toSeconds(ms);

    // Assert
    expect(seconds).toBe(0.2);
  });
});

describe("usePrefersReducedMotion", () => {
  it("true, когда система просит уменьшить анимацию", () => {
    // Arrange
    stubMatchMedia(["(prefers-reduced-motion: reduce)"]);

    // Act
    const { result } = renderHook(() => usePrefersReducedMotion());

    // Assert
    expect(result.current).toBe(true);
  });

  it("false по умолчанию", () => {
    // Arrange
    stubMatchMedia();

    // Act
    const { result } = renderHook(() => usePrefersReducedMotion());

    // Assert
    expect(result.current).toBe(false);
  });
});
