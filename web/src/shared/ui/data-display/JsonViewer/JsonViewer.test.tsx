import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { type JsonValue, JsonViewer } from "~/shared/ui/data-display";

const VALUE: JsonValue = { name: "left-pad", author: { login: "azer", id: 7 }, tags: ["a", "b"] };

describe("JsonViewer", () => {
  it("по умолчанию показывает все узлы с номерами строк", () => {
    // Arrange
    // Act
    render(<JsonViewer value={VALUE} />);

    // Assert
    expect(screen.getByText('"login"')).toBeInTheDocument();
    expect(screen.getByText("11")).toBeInTheDocument();
  });

  it("сворачивает узел: вложенные ключи скрыты, кнопка предлагает развернуть", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<JsonViewer value={VALUE} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Свернуть author" }));

    // Assert
    expect(screen.queryByText('"login"')).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Развернуть author" })).toHaveAttribute(
      "aria-expanded",
      "false",
    );
  });

  it("свёрнутый узел показывает число ключей", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<JsonViewer value={VALUE} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Свернуть author" }));

    // Assert
    expect(screen.getByText("2 ключа")).toBeInTheDocument();
  });

  it("повторное нажатие разворачивает узел обратно", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<JsonViewer value={VALUE} />);
    await user.click(screen.getByRole("button", { name: "Свернуть author" }));

    // Act
    await user.click(screen.getByRole("button", { name: "Развернуть author" }));

    // Assert
    expect(screen.getByText('"login"')).toBeInTheDocument();
  });

  it("«Свернуть всё» оставляет только корень", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<JsonViewer value={VALUE} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Свернуть всё" }));

    // Assert
    expect(screen.queryByText('"name"')).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Развернуть корень" })).toBeInTheDocument();
  });

  it("значение с разметкой выводится текстом, а не элементом", () => {
    // Arrange
    const value = { note: "<b>жирный</b>" };

    // Act
    render(<JsonViewer value={value} />);

    // Assert
    expect(screen.getByText('"<b>жирный</b>"')).toBeInTheDocument();
  });
});
