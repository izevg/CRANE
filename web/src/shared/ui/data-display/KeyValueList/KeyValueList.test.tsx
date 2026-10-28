import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { KeyValueList } from "~/shared/ui/data-display";

const REPO_NAME = "main";

describe("KeyValueList", () => {
  it("выводит пары «свойство — значение» в списке определений", () => {
    // Arrange
    const items = [
      { label: "Формат", value: "npm" },
      { label: "Репозиторий", value: <a href="/repos/main">{REPO_NAME}</a> },
    ];

    // Act
    render(<KeyValueList items={items} />);

    // Assert
    expect(screen.getAllByRole("term").map((term) => term.textContent)).toEqual([
      "Формат",
      "Репозиторий",
    ]);
    expect(screen.getAllByRole("definition").map((value) => value.textContent)).toEqual([
      "npm",
      "main",
    ]);
  });

  it("пустое значение заменяется прочерком", () => {
    // Arrange
    const items = [{ label: "Описание", value: null }];

    // Act
    render(<KeyValueList items={items} />);

    // Assert
    expect(screen.getByRole("definition")).toHaveTextContent("—");
  });
});
