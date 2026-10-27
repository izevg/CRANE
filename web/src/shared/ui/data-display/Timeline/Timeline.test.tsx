import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Timeline } from "~/shared/ui/data-display";

describe("Timeline", () => {
  it("события выводятся упорядоченным списком в заданном порядке", () => {
    // Arrange
    const items = [
      {
        id: "b",
        time: "14:07",
        title: "Приоритет повышен до P1",
        tone: "warning",
        tag: "по правилу",
      },
      { id: "a", time: "13:42", title: "Находка создана" },
    ] as const;

    // Act
    render(<Timeline label="История приоритета" items={items} />);

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

  it("тег события отображается рядом с заголовком", () => {
    // Arrange
    const items = [
      { id: "b", time: "14:07", title: "Приоритет повышен", tone: "error", tag: "оценка модели" },
    ] as const;

    // Act
    render(<Timeline label="История" items={items} />);

    // Assert
    expect(screen.getByText("оценка модели")).toBeVisible();
  });

  it("ссылка и подпись события выводятся, когда переданы", () => {
    // Arrange
    const items = [
      {
        id: "a",
        time: "13:42",
        title: "Создана задача",
        caption: "Назначена команде web",
        link: { label: "#4521", run: { href: "/issues/4521" } },
      },
    ] as const;

    // Act
    render(<Timeline label="История" items={items} />);

    // Assert
    expect(screen.getByRole("link", { name: "#4521" })).toHaveAttribute("href", "/issues/4521");
    expect(screen.getByText("Назначена команде web")).toBeVisible();
  });
});
