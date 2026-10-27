import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SecretOnceDialog } from "~/shared/ui/dialogs";

const LENGTH_HINT = /Длина/;
const SECRET = "crane_pat_9f3kQ2vXr7LmT0aB5nHcYe1uWdZs8GjP";

describe("SecretOnceDialog", () => {
  it("показывает предупреждение, секрет целиком и кнопку копирования", () => {
    // Arrange
    // Act
    render(<SecretOnceDialog open secret={SECRET} title="Токен выпущен" onDone={vi.fn()} />);

    // Assert
    expect(screen.getByText("Сохраните сейчас — больше мы его не покажем")).toBeVisible();
    expect(screen.getByText(SECRET)).toBeVisible();
    expect(screen.getByRole("button", { name: "Копировать токен" })).toBeVisible();
  });

  it("не показывает длину секрета", () => {
    // Arrange
    // Act
    render(<SecretOnceDialog open secret={SECRET} title="Токен выпущен" onDone={vi.fn()} />);

    // Assert
    expect(screen.queryByText(LENGTH_HINT)).not.toBeInTheDocument();
  });

  it("«Готово» недоступна до отметки чекбокса", () => {
    // Arrange
    // Act
    render(<SecretOnceDialog open secret={SECRET} title="Токен выпущен" onDone={vi.fn()} />);

    // Assert
    expect(screen.getByRole("button", { name: "Готово" })).toHaveAttribute("aria-disabled", "true");
  });

  it("нажатие на недоступную «Готово» не закрывает диалог", async () => {
    // Arrange
    const user = userEvent.setup();
    const onDone = vi.fn();
    render(<SecretOnceDialog open secret={SECRET} title="Токен выпущен" onDone={onDone} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Готово" }));

    // Assert
    expect(onDone).not.toHaveBeenCalled();
  });

  it("Esc до отметки не закрывает диалог", async () => {
    // Arrange
    const user = userEvent.setup();
    const onDone = vi.fn();
    render(<SecretOnceDialog open secret={SECRET} title="Токен выпущен" onDone={onDone} />);

    // Act
    await user.keyboard("{Escape}");

    // Assert
    expect(onDone).not.toHaveBeenCalled();
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
  });

  it("после отметки «Готово» становится доступной и вызывает onDone", async () => {
    // Arrange
    const user = userEvent.setup();
    const onDone = vi.fn();
    render(<SecretOnceDialog open secret={SECRET} title="Токен выпущен" onDone={onDone} />);
    await user.click(screen.getByRole("checkbox", { name: "Я сохранил токен" }));

    // Act
    await user.click(screen.getByRole("button", { name: "Готово" }));

    // Assert
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it("после отметки Esc закрывает диалог", async () => {
    // Arrange
    const user = userEvent.setup();
    const onDone = vi.fn();
    render(<SecretOnceDialog open secret={SECRET} title="Токен выпущен" onDone={onDone} />);
    await user.click(screen.getByRole("checkbox", { name: "Я сохранил токен" }));

    // Act
    await user.keyboard("{Escape}");

    // Assert
    expect(onDone).toHaveBeenCalledTimes(1);
  });

  it("при open=false секрета нет в документе", () => {
    // Arrange
    const { rerender } = render(
      <SecretOnceDialog open secret={SECRET} title="Токен выпущен" onDone={vi.fn()} />,
    );

    // Act
    rerender(
      <SecretOnceDialog open={false} secret={SECRET} title="Токен выпущен" onDone={vi.fn()} />,
    );

    // Assert
    expect(screen.queryByText(SECRET)).not.toBeInTheDocument();
    expect(document.body).not.toHaveTextContent(SECRET);
  });

  it("повторное открытие начинается с неотмеченного чекбокса", () => {
    // Arrange
    const { rerender } = render(
      <SecretOnceDialog open secret={SECRET} title="Токен выпущен" onDone={vi.fn()} />,
    );
    rerender(
      <SecretOnceDialog open={false} secret={SECRET} title="Токен выпущен" onDone={vi.fn()} />,
    );

    // Act
    rerender(<SecretOnceDialog open secret={SECRET} title="Токен выпущен" onDone={vi.fn()} />);

    // Assert
    expect(screen.getByRole("button", { name: "Готово" })).toHaveAttribute("aria-disabled", "true");
  });
});
