import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { type DiffRow, DiffViewer } from "~/shared/ui/data-display";

const ADDED = /\^4\.17\.21/;
const REMOVED = /\^4\.17\.20/;
const SAME = /"app"/;

const ROWS: readonly DiffRow[] = [
  { kind: "same", text: '"name": "app"', path: "name" },
  { kind: "remove", text: '"lodash": "^4.17.20"', path: "dependencies" },
  { kind: "add", text: '"lodash": "^4.17.21"', path: "dependencies" },
];

describe("DiffViewer", () => {
  it("добавленная строка помечена знаком «+» и текстом «добавлено» для скринридера", () => {
    // Arrange
    // Act
    render(<DiffViewer rows={ROWS} label="Манифест" />);

    // Assert
    const row = screen.getByRole("row", { name: ADDED });
    expect(row).toHaveTextContent("+");
    expect(row).toHaveTextContent("добавлено");
  });

  it("удалённая строка помечена знаком «−» и текстом «удалено»", () => {
    // Arrange
    // Act
    render(<DiffViewer rows={ROWS} label="Манифест" />);

    // Assert
    const row = screen.getByRole("row", { name: REMOVED });
    expect(row).toHaveTextContent("−");
    expect(row).toHaveTextContent("удалено");
  });

  it("неизменная строка не получает пометки", () => {
    // Arrange
    // Act
    render(<DiffViewer rows={ROWS} label="Манифест" />);

    // Assert
    const row = screen.getByRole("row", { name: SAME });
    expect(row).not.toHaveTextContent("добавлено");
    expect(row).not.toHaveTextContent("удалено");
  });

  it("таблица получает доступное имя из label", () => {
    // Arrange
    // Act
    render(<DiffViewer rows={ROWS} label="Манифест 1.4.1 → 1.4.2" />);

    // Assert
    expect(screen.getByRole("table", { name: "Манифест 1.4.1 → 1.4.2" })).toBeInTheDocument();
  });

  it("режим split показывает колонки «Было» и «Стало»", () => {
    // Arrange
    // Act
    render(<DiffViewer rows={ROWS} mode="split" label="Манифест" />);

    // Assert
    expect(screen.getByRole("columnheader", { name: "Было" })).toBeVisible();
    expect(screen.getByRole("columnheader", { name: "Стало" })).toBeVisible();
  });

  it("в режиме split добавленная строка остаётся помеченной", () => {
    // Arrange
    // Act
    render(<DiffViewer rows={ROWS} mode="split" label="Манифест" />);

    // Assert
    expect(screen.getByRole("row", { name: ADDED })).toHaveTextContent("добавлено");
  });
});
