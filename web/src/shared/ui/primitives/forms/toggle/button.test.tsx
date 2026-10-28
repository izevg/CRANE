import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Button } from "~/shared/ui/primitives";

describe("Button", () => {
  it("вызывает обработчик по клику", async () => {
    // Arrange
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(<Button onClick={onClick}>{"Сохранить"}</Button>);

    // Act
    await user.click(screen.getByRole("button", { name: "Сохранить" }));

    // Assert
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("отключённая кнопка не реагирует на клик", async () => {
    // Arrange
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(
      <Button disabled onClick={onClick}>
        {"Сохранить"}
      </Button>,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Сохранить" }));

    // Assert
    expect(onClick).not.toHaveBeenCalled();
  });

  it("иконочная кнопка получает имя из aria-label", () => {
    // Arrange
    const label = "Закрыть панель";

    // Act
    render(<Button size="icon-sm" aria-label={label} />);

    // Assert
    expect(screen.getByRole("button", { name: label })).toBeInTheDocument();
  });

  it("pending: клик не срабатывает, кнопка занята и остаётся в порядке фокуса", async () => {
    // Arrange
    const user = userEvent.setup();
    const onClick = vi.fn();
    render(
      <Button pending onClick={onClick}>
        {"Повторить"}
      </Button>,
    );
    const button = screen.getByRole("button", { name: "Повторить" });

    // Act
    await user.click(button);

    // Assert
    expect(onClick).not.toHaveBeenCalled();
    expect(button).toHaveAttribute("aria-busy", "true");
    expect(button).toHaveAttribute("aria-disabled", "true");
    expect(button.querySelector("svg")).not.toBeNull();
  });

  it("без pending индикатора и aria-busy нет", () => {
    // Arrange
    // Act
    render(<Button>{"Повторить"}</Button>);

    // Assert
    const button = screen.getByRole("button", { name: "Повторить" });
    expect(button).not.toHaveAttribute("aria-busy");
    expect(button.querySelector("svg")).toBeNull();
  });
});
