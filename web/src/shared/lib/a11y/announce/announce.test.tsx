import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { announce, LiveRegion } from "~/shared/lib/a11y";

const MESSAGE = "Скопировано";
const REWRITE_MS = 100;

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  vi.useRealTimers();
});

describe("announce", () => {
  it("записывает сообщение в polite live-регион", async () => {
    // Arrange
    render(<LiveRegion />);

    // Act
    await act(async () => {
      announce(MESSAGE);
      await vi.advanceTimersByTimeAsync(REWRITE_MS);
    });

    // Assert
    expect(screen.getByRole("status")).toHaveTextContent(MESSAGE);
    expect(screen.getByRole("status")).toHaveAttribute("aria-live", "polite");
  });

  it("повтор того же текста сначала очищает регион, чтобы скринридер объявил его снова", async () => {
    // Arrange
    render(<LiveRegion />);
    act(() => announce(MESSAGE));
    await act(() => vi.advanceTimersByTimeAsync(REWRITE_MS));

    // Act
    act(() => announce(MESSAGE));

    // Assert
    expect(screen.getByRole("status")).toBeEmptyDOMElement();
    await act(() => vi.advanceTimersByTimeAsync(REWRITE_MS));
    expect(screen.getByRole("status")).toHaveTextContent(MESSAGE);
  });
});
