import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Action } from "~/shared/ui/actions";
import { PageHeader } from "~/shared/ui/layout";

const open = (label: string): Action => ({ label, run: { onSelect: vi.fn() } });

describe("PageHeader", () => {
  it("заголовок страницы — h1 с именем и программным фокусом", () => {
    // Arrange
    // Act
    render(<PageHeader title="left-pad" />);

    // Assert
    const heading = screen.getByRole("heading", { level: 1, name: "left-pad" });
    expect(heading).toHaveAttribute("tabindex", "-1");
  });

  it("заголовок можно сфокусировать из приложения при смене маршрута", () => {
    // Arrange
    render(<PageHeader title="left-pad" />);
    const heading = screen.getByRole("heading", { level: 1 });

    // Act
    heading.focus();

    // Assert
    expect(heading).toHaveFocus();
  });

  it("headingId задаёт id заголовка", () => {
    // Arrange
    // Act
    render(<PageHeader title="left-pad" headingId="page-title" />);

    // Assert
    expect(screen.getByRole("heading", { level: 1 })).toHaveAttribute("id", "page-title");
  });

  it("без действий нет ни кнопок, ни ссылок", () => {
    // Arrange
    // Act
    render(<PageHeader title="left-pad" />);

    // Assert
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
  });

  it("primary и secondary рендерятся только когда переданы Action", () => {
    // Arrange
    // Act
    render(
      <PageHeader
        title="left-pad"
        primary={open("Заблокировать")}
        secondary={[open("Экспорт"), { label: "Открыть", run: { href: "/x" } }]}
      />,
    );

    // Assert
    expect(screen.getByRole("button", { name: "Заблокировать" })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Экспорт" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Открыть" })).toHaveAttribute("href", "/x");
    expect(screen.queryByRole("button", { name: "Другие действия" })).not.toBeInTheDocument();
  });

  it("клик по primary вызывает обработчик действия", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<PageHeader title="left-pad" primary={{ label: "Заблокировать", run: { onSelect } }} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Заблокировать" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("меню «…» ставит опасное действие последним", async () => {
    // Arrange
    const user = userEvent.setup();
    const menu: readonly Action[] = [
      { label: "Удалить", run: { onSelect: vi.fn() }, destructive: true },
      open("Копировать ссылку"),
    ];
    render(<PageHeader title="left-pad" menu={menu} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Другие действия" }));

    // Assert
    const items = await screen.findAllByRole("menuitem");
    expect(items.map((item) => item.textContent)).toEqual(["Копировать ссылку", "Удалить"]);
  });

  it("статус, чипы, подпись, статистика, вкладки и aside выводятся слотами", () => {
    // Arrange
    // Act
    render(
      <PageHeader
        title="left-pad"
        status={<span>{"Опубликован"}</span>}
        chips={<span>{"npm"}</span>}
        caption="Пакет из репозитория npm-proxy"
        stats={<span>{"Версий 48"}</span>}
        tabs={<div role="tablist" aria-label="Разделы" />}
        aside={<span>{"Окно 24 ч"}</span>}
      />,
    );

    // Assert
    for (const text of [
      "Опубликован",
      "npm",
      "Пакет из репозитория npm-proxy",
      "Версий 48",
      "Окно 24 ч",
    ]) {
      expect(screen.getByText(text)).toBeInTheDocument();
    }
    expect(screen.getByRole("tablist", { name: "Разделы" })).toBeInTheDocument();
  });
});
