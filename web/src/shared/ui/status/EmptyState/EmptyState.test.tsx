import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { EmptyState } from "~/shared/ui/status";

describe("EmptyState", () => {
  it("first-run показывает заголовок и описание", () => {
    // Arrange
    const title = "Пакетов пока нет";

    // Act
    render(<EmptyState variant="first-run" title={title} description="Здесь появятся пакеты" />);

    // Assert
    expect(screen.getByRole("heading", { level: 2, name: title })).toBeInTheDocument();
    expect(screen.getByText("Здесь появятся пакеты")).toBeInTheDocument();
  });

  it("first-run без действий не рисует кнопок и ссылок", () => {
    // Arrange
    const title = "Пакетов пока нет";

    // Act
    render(<EmptyState variant="first-run" title={title} />);

    // Assert
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
  });

  it("first-run с primary и secondary рисует обе, primary вызывает обработчик", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <EmptyState
        variant="first-run"
        title="Пакетов пока нет"
        primary={{ label: "Как опубликовать", run: { onSelect } }}
        secondary={{ label: "Импорт из Nexus", run: { href: "/import" } }}
      />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Как опубликовать" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("link", { name: "Импорт из Nexus" })).toBeInTheDocument();
  });

  it("first-run с одним secondary не рисует primary", () => {
    // Arrange
    const secondary = { label: "Импорт из Nexus", run: { href: "/import" } } as const;

    // Act
    render(<EmptyState variant="first-run" title="Пакетов пока нет" secondary={secondary} />);

    // Assert
    expect(screen.queryByRole("button", { name: "Как опубликовать" })).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Импорт из Nexus" })).toBeInTheDocument();
  });

  it("filtered по умолчанию показывает стандартный заголовок", () => {
    // Arrange
    const variant = "filtered";

    // Act
    render(<EmptyState variant={variant} />);

    // Assert
    expect(
      screen.getByRole("heading", { name: "Ничего не найдено по фильтрам" }),
    ).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("filtered с reset рисует «Сбросить фильтры», и клик вызывает обработчик", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <EmptyState variant="filtered" reset={{ label: "Сбросить фильтры", run: { onSelect } }} />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Сбросить фильтры" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("заголовок получает уровень из headingLevel", () => {
    // Arrange
    const title = "Пакетов пока нет";

    // Act
    render(<EmptyState variant="first-run" title={title} headingLevel={3} />);

    // Assert
    expect(screen.getByRole("heading", { level: 3, name: title })).toBeInTheDocument();
  });
});
