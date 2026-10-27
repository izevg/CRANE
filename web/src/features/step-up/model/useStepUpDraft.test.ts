import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";
import { useStepUpDraft } from "./useStepUpDraft";

const isText = (value: unknown): value is string => typeof value === "string";

afterEach(() => {
  sessionStorage.clear();
});

describe("useStepUpDraft", () => {
  it("хранит черновик под ключом с префиксом step-up", () => {
    // Arrange
    const { result } = renderHook(() => useStepUpDraft("reveal-event", isText));

    // Act
    act(() => result.current[1]("обоснование"));

    // Assert
    expect(sessionStorage.getItem("crane.stepup.reveal-event")).toBe("обоснование");
  });

  it("после редиректа черновик восстанавливается по тому же formId", () => {
    // Arrange
    sessionStorage.setItem("crane.stepup.reveal-event", "обоснование");

    // Act
    const { result } = renderHook(() => useStepUpDraft("reveal-event", isText));

    // Assert
    expect(result.current[0]).toBe("обоснование");
  });
});
