import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { RefreshFailed } from "./Notices";

describe("RefreshFailed", () => {
  it("во время повтора «Повторить» занята: aria-busy, индикатор, клик не срабатывает", async () => {
    // Arrange
    const user = userEvent.setup();
    const onRetry = vi.fn();
    render(<RefreshFailed onRetry={onRetry} retrying failedAt={1} />);
    const button = screen.getByRole("button", { name: "Повторить" });

    // Act
    await user.click(button);

    // Assert
    expect(button).toHaveAttribute("aria-busy", "true");
    expect(button.querySelector("svg")).not.toBeNull();
    expect(onRetry).not.toHaveBeenCalled();
  });

  it("между повторами кнопка свободна и вызывает повтор", async () => {
    // Arrange
    const user = userEvent.setup();
    const onRetry = vi.fn();
    render(<RefreshFailed onRetry={onRetry} retrying={false} failedAt={1} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Повторить" }));

    // Assert
    expect(onRetry).toHaveBeenCalledTimes(1);
  });
});
