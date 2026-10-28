import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { Topbar } from "./Topbar";

const CRUMBS = {
  trail: [
    { label: "Пакеты", href: "/packages" },
    { label: "left-pad", href: "/packages/left-pad" },
  ],
  current: "1.3.0",
};

describe("Topbar", () => {
  it("последняя крошка — текущая страница без ссылки", () => {
    // Arrange
    // Act
    render(<Topbar breadcrumbs={CRUMBS} onOpenPalette={vi.fn()} sidebarToggle={null} />);

    // Assert
    const current = screen.getByText("1.3.0");
    expect(current).toHaveAttribute("aria-current", "page");
    expect(screen.queryByRole("link", { name: "1.3.0" })).not.toBeInTheDocument();
  });

  it("предыдущие крошки — ссылки на свои адреса", () => {
    // Arrange
    // Act
    render(<Topbar breadcrumbs={CRUMBS} onOpenPalette={vi.fn()} sidebarToggle={null} />);

    // Assert
    expect(screen.getByRole("link", { name: "Пакеты" })).toHaveAttribute("href", "/packages");
    expect(screen.getByRole("link", { name: "left-pad" })).not.toHaveAttribute("aria-current");
  });

  it("без onOpenPalette кнопки поиска нет", () => {
    // Arrange
    // Act
    render(<Topbar breadcrumbs={CRUMBS} sidebarToggle={null} />);

    // Assert
    expect(
      screen.queryByRole("button", { name: "Поиск и переходы Ctrl K" }),
    ).not.toBeInTheDocument();
  });

  it("кнопка поиска вызывает onOpenPalette и называет сочетание клавиш", async () => {
    // Arrange
    const user = userEvent.setup();
    const onOpenPalette = vi.fn();
    render(<Topbar breadcrumbs={CRUMBS} onOpenPalette={onOpenPalette} sidebarToggle={null} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Поиск и переходы Ctrl K" }));

    // Assert
    expect(onOpenPalette).toHaveBeenCalledTimes(1);
  });

  it("слоты отрисованы в порядке: кнопка меню, поиск, workspace, статус, уведомления, аккаунт", () => {
    // Arrange
    // Act
    render(
      <Topbar
        breadcrumbs={CRUMBS}
        onOpenPalette={vi.fn()}
        sidebarToggle={<button type="button">{"Меню"}</button>}
        workspace={<button type="button">{"Workspace"}</button>}
        status={<span>{"Режим: норма"}</span>}
        notifications={<button type="button">{"Уведомления"}</button>}
        account={<button type="button">{"Аккаунт"}</button>}
      />,
    );

    // Assert
    const names = screen.getAllByRole("button").map((button) => button.textContent);
    expect(names).toEqual([
      "Меню",
      "Поиск и переходы Ctrl K",
      "Workspace",
      "Уведомления",
      "Аккаунт",
    ]);
    expect(screen.getByText("Режим: норма")).toBeInTheDocument();
  });
});
