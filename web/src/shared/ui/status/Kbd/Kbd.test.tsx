import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Kbd } from "~/shared/ui/status";

describe("Kbd", () => {
  it("рендерит клавишу семантическим элементом kbd", () => {
    // Arrange
    const key = "Ctrl";

    // Act
    render(<Kbd>{key}</Kbd>);

    // Assert
    expect(screen.getByText(key).tagName).toBe("KBD");
  });
});
