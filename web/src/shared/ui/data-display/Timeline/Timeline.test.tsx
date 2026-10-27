import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Timeline } from "~/shared/ui/data-display";

const ITEMS = [
  {
    id: "b",
    time: "2026-10-27T14:07:00Z",
    title: "Приоритет повышен до P1",
    tone: "warning",
    status: { tone: "warning", label: "P1" },
    source: "по правилу",
  },
  { id: "a", time: "2026-10-27T13:42:00Z", title: "Находка создана", actor: "Иванов И." },
] as const;

describe("Timeline", () => {
  it("события выводятся упорядоченным списком в заданном порядке", () => {
    // Arrange
    // Act
    render(<Timeline label="История приоритета" items={ITEMS} />);

    // Assert
    const list = screen.getByRole("list", { name: "История приоритета" });
    expect(list.tagName).toBe("OL");
    expect(
      within(list)
        .getAllByRole("listitem")
        .map((item) => item.textContent),
    ).toEqual([
      expect.stringContaining("Приоритет повышен до P1"),
      expect.stringContaining("Находка создана"),
    ]);
  });

  it("статус и источник выводятся отдельно: чип статуса и чип источника", () => {
    // Arrange
    // Act
    render(<Timeline label="История" items={ITEMS} />);

    // Assert
    expect(screen.getByText("P1")).toBeVisible();
    expect(screen.getByText("по правилу")).toBeVisible();
  });

  it("время выводится элементом time с машиночитаемым значением", () => {
    // Arrange
    // Act
    render(<Timeline label="История" items={ITEMS} />);

    // Assert
    const [first] = screen.getAllByText((_, element) => element?.tagName === "TIME");
    expect(first).toHaveAttribute("datetime", "2026-10-27T14:07:00.000Z");
  });

  it("актор выводится рядом со временем", () => {
    // Arrange
    // Act
    render(<Timeline label="История" items={ITEMS} />);

    // Assert
    expect(screen.getByText("Иванов И.")).toBeVisible();
  });

  it("ссылка рендерится текстовой ссылкой, а не кнопкой", () => {
    // Arrange
    const items = [
      {
        id: "a",
        time: "2026-10-27T13:42:00Z",
        title: "Создана задача",
        link: { label: "#4521", run: { href: "/issues/4521" } },
      },
    ] as const;

    // Act
    render(<Timeline label="История" items={items} />);

    // Assert
    expect(screen.getByRole("link", { name: "#4521" })).toHaveAttribute("href", "/issues/4521");
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("деталь выводится под заголовком", () => {
    // Arrange
    const items = [
      { id: "a", time: "2026-10-27T13:42:00Z", title: "Скан", detail: "Найдена 1 уязвимость" },
    ] as const;

    // Act
    render(<Timeline label="История" items={items} />);

    // Assert
    expect(screen.getByText("Найдена 1 уязвимость")).toBeVisible();
  });
});
