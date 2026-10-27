import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Switch } from "~/shared/ui/primitives";

describe("Switch", () => {
  it("переключается по клику", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Switch aria-label="Только мои" />);

    // Act
    await user.click(screen.getByRole("switch", { name: "Только мои" }));

    // Assert
    expect(screen.getByRole("switch", { name: "Только мои" })).toBeChecked();
  });

  it("переключается с клавиатуры пробелом", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Switch size="sm" aria-label="Автообновление" />);
    await user.tab();

    // Act
    await user.keyboard(" ");

    // Assert
    expect(screen.getByRole("switch", { name: "Автообновление" })).toBeChecked();
  });
});
