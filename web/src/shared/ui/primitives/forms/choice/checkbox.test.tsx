import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Checkbox } from "~/shared/ui/primitives";

describe("Checkbox", () => {
  it("отмечается по клику", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Checkbox aria-label="Выбрать строку" />);

    // Act
    await user.click(screen.getByRole("checkbox", { name: "Выбрать строку" }));

    // Assert
    expect(screen.getByRole("checkbox", { name: "Выбрать строку" })).toBeChecked();
  });

  it("неопределённое состояние отдаётся как mixed", () => {
    // Arrange
    const label = "Выбрать все";

    // Act
    render(<Checkbox indeterminate aria-label={label} />);

    // Assert
    expect(screen.getByRole("checkbox", { name: label })).toBePartiallyChecked();
  });
});
