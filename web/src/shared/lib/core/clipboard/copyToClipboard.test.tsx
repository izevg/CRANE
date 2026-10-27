import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LiveRegion } from "~/shared/lib/a11y";
import { copyToClipboard } from "~/shared/lib/core";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("copyToClipboard", () => {
  it("копирует значение и объявляет «Скопировано»", async () => {
    // Arrange
    userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
    render(<LiveRegion />);

    // Act
    const copied = await copyToClipboard("abc");

    // Assert
    expect(copied).toBe(true);
    expect(writeText).toHaveBeenCalledWith("abc");
    expect(await screen.findByText("Скопировано")).toBeInTheDocument();
  });

  it("возвращает false и ничего не объявляет, если запись отклонена", async () => {
    // Arrange
    userEvent.setup();
    vi.spyOn(navigator.clipboard, "writeText").mockRejectedValue(new Error("denied"));
    render(<LiveRegion />);

    // Act
    const copied = await copyToClipboard("abc");

    // Assert
    expect(copied).toBe(false);
    expect(screen.getByRole("status")).toBeEmptyDOMElement();
  });
});
