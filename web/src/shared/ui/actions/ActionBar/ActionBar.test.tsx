import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { type Action, ActionBar } from "~/shared/ui/actions";

const act = (label: string): Action => ({ label, run: { onSelect: vi.fn() } });

describe("ActionBar", () => {
  it("пустая группа не рисует ничего", () => {
    // Arrange
    // Act
    const { container } = render(<ActionBar actions={{}} />);

    // Assert
    expect(container).toBeEmptyDOMElement();
  });

  it("группа с пустым меню и пустым secondary не рисует ничего", () => {
    // Arrange
    // Act
    const { container } = render(<ActionBar actions={{ menu: [] }} />);

    // Assert
    expect(container).toBeEmptyDOMElement();
  });

  it("пустой плоский ряд не рисует ничего", () => {
    // Arrange
    // Act
    const { container } = render(<ActionBar actions={[]} />);

    // Assert
    expect(container).toBeEmptyDOMElement();
  });

  it("primary и secondary рисуются кнопками только для переданных Action", () => {
    // Arrange
    // Act
    render(
      <ActionBar
        actions={{ primary: act("Сохранить"), secondary: [act("Отмена"), act("Черновик")] }}
      />,
    );

    // Assert
    expect(screen.getAllByRole("button").map((button) => button.textContent)).toEqual([
      "Сохранить",
      "Отмена",
      "Черновик",
    ]);
  });

  it("без primary и menu есть только secondary, меню «…» отсутствует", () => {
    // Arrange
    // Act
    render(<ActionBar actions={{ secondary: [act("Отмена")] }} />);

    // Assert
    expect(screen.getAllByRole("button")).toHaveLength(1);
    expect(screen.queryByRole("button", { name: "Другие действия" })).not.toBeInTheDocument();
  });

  it("menu открывается кнопкой «…» и показывает действия", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<ActionBar actions={{ menu: [act("Копировать ссылку")] }} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Другие действия" }));

    // Assert
    expect(await screen.findByRole("menuitem", { name: "Копировать ссылку" })).toBeInTheDocument();
  });

  it("действие с href рисуется ссылкой", () => {
    // Arrange
    // Act
    render(<ActionBar actions={{ primary: { label: "Открыть", run: { href: "/p" } } }} />);

    // Assert
    expect(screen.getByRole("link", { name: "Открыть" })).toHaveAttribute("href", "/p");
  });

  it("плоский массив рисует ряд кнопок любой длины", () => {
    // Arrange
    // Act
    render(<ActionBar actions={[act("А"), act("Б"), act("В")]} />);

    // Assert
    expect(screen.getAllByRole("button").map((button) => button.textContent)).toEqual([
      "А",
      "Б",
      "В",
    ]);
  });
});
