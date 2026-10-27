import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { AutoRefreshToggle } from "~/shared/ui/actions";

const NAME = "Пауза автообновления";

describe("AutoRefreshToggle", () => {
  it("кнопка не нажата, пока автообновление идёт", () => {
    // Arrange
    const paused = false;

    // Act
    render(<AutoRefreshToggle paused={paused} onPausedChange={vi.fn()} />);

    // Assert
    expect(screen.getByRole("button", { name: NAME })).toHaveAttribute("aria-pressed", "false");
  });

  it("кнопка нажата, когда автообновление на паузе", () => {
    // Arrange
    const paused = true;

    // Act
    render(<AutoRefreshToggle paused={paused} onPausedChange={vi.fn()} />);

    // Assert
    expect(screen.getByRole("button", { name: NAME })).toHaveAttribute("aria-pressed", "true");
  });

  it("нажатие просит поставить на паузу", async () => {
    // Arrange
    const user = userEvent.setup();
    const onPausedChange = vi.fn();
    render(<AutoRefreshToggle paused={false} onPausedChange={onPausedChange} />);

    // Act
    await user.click(screen.getByRole("button", { name: NAME }));

    // Assert
    expect(onPausedChange).toHaveBeenCalledExactlyOnceWith(true);
  });

  it("нажатие на паузе просит возобновить", async () => {
    // Arrange
    const user = userEvent.setup();
    const onPausedChange = vi.fn();
    render(<AutoRefreshToggle paused={true} onPausedChange={onPausedChange} />);

    // Act
    await user.click(screen.getByRole("button", { name: NAME }));

    // Assert
    expect(onPausedChange).toHaveBeenCalledExactlyOnceWith(false);
  });
});
