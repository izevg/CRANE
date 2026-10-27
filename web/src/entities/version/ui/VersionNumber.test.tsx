import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { VersionNumber } from "../index";

describe("VersionNumber", () => {
  it("номер обычной версии не зачёркнут", () => {
    // Arrange
    // Act
    render(<VersionNumber name="1.4.1" state="released" />);

    // Assert
    expect(screen.getByText("1.4.1").tagName).toBe("SPAN");
  });

  it("у удалённой версии номер зачёркнут", () => {
    // Arrange
    // Act
    render(<VersionNumber name="1.3.9" state="deleted" />);

    // Assert
    expect(screen.getByText("1.3.9").tagName).toBe("S");
  });

  it("пояснение про повторное использование входит в доступное описание номера", () => {
    // Arrange
    // Act
    render(<VersionNumber name="1.3.9" state="deleted" />);

    // Assert
    expect(screen.getByText("1.3.9")).toHaveAccessibleDescription(
      "Номер версии нельзя использовать повторно",
    );
  });

  it("у удалённой версии подсказка объясняет, что номер нельзя использовать повторно", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<VersionNumber name="1.3.9" state="deleted" />);

    // Act
    await user.tab();

    // Assert
    expect(
      (await screen.findAllByText("Номер версии нельзя использовать повторно")).length,
    ).toBeGreaterThan(0);
  });
});
