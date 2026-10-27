import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { CodeBlock } from "~/shared/ui/data-display";

const INSTALL_LINE = /pnpm add @acme\/ui@1\.4\.2/;
const COMMAND = "pnpm add @acme/ui@1.4.2\npnpm install";

describe("CodeBlock", () => {
  it("кнопка копирования получает имя из copyLabel", () => {
    // Arrange
    // Act
    render(<CodeBlock code={COMMAND} copyLabel="Копировать команду установки" />);

    // Assert
    expect(screen.getByRole("button", { name: "Копировать команду установки" })).toBeVisible();
  });

  it("копирует полный код, включая все строки", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<CodeBlock code={COMMAND} copyLabel="Копировать команду установки" />);

    // Act
    await user.click(screen.getByRole("button", { name: "Копировать команду установки" }));

    // Assert
    expect(await navigator.clipboard.readText()).toBe(COMMAND);
  });

  it("показывает код текстом", () => {
    // Arrange
    // Act
    render(<CodeBlock code={COMMAND} copyLabel="Копировать" wrap />);

    // Assert
    expect(screen.getByText(INSTALL_LINE)).toBeInTheDocument();
  });
});
