import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { ToggleGroup, ToggleGroupItem } from "~/shared/ui/primitives";

describe("ToggleGroup", () => {
  it("нажатие на элемент помечает его нажатым", async () => {
    // Arrange
    const user = userEvent.setup();
    render(
      <ToggleGroup variant="outline" size="sm" aria-label="Формат">
        <ToggleGroupItem value="npm">{"npm"}</ToggleGroupItem>
        <ToggleGroupItem value="maven">{"maven"}</ToggleGroupItem>
      </ToggleGroup>,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "maven" }));

    // Assert
    expect(screen.getByRole("button", { name: "maven" })).toHaveAttribute("aria-pressed", "true");
    expect(screen.getByRole("button", { name: "npm" })).toHaveAttribute("aria-pressed", "false");
  });

  it("в одиночном режиме выбор нового элемента снимает прежний", async () => {
    // Arrange
    const user = userEvent.setup();
    render(
      <ToggleGroup defaultValue={["npm"]} aria-label="Формат">
        <ToggleGroupItem value="npm">{"npm"}</ToggleGroupItem>
        <ToggleGroupItem value="maven">{"maven"}</ToggleGroupItem>
      </ToggleGroup>,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "maven" }));

    // Assert
    expect(screen.getByRole("button", { name: "npm" })).toHaveAttribute("aria-pressed", "false");
  });
});
