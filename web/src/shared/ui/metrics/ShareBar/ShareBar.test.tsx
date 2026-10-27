// biome-ignore-all lint/style/noMagicNumbers: тестовые объёмы — произвольные числа
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { textFormat } from "~/shared/lib/text";
import { ShareBar } from "~/shared/ui/metrics";

// Intl ставит неразрывный пробел перед «%»; нормализатор Testing Library по умолчанию его свернул бы.
const percent = textFormat("ru").percent;
const RAW = { normalizer: (text: string) => text };
const PLATFORM_ROW = /Платформа/;
const OTHER_ROW = /Прочие \(2\)/;
const ANY_OTHER = /Прочие/;
const withGb = (sum: number) => `${sum} ГБ`;
const COLUMNS = { name: "Команда", value: "Объём", share: "Доля" };
const TITLE = "Объём по командам";

function makeItems(count: number) {
  return Array.from({ length: count }, (_, index) => ({
    id: `team-${index}`,
    label: `Команда ${index + 1}`,
    value: count - index,
    display: `${count - index} ГБ`,
  }));
}

describe("ShareBar", () => {
  it("легенда — таблица с заголовками колонок из пропсов", () => {
    // Arrange
    const items = makeItems(3);

    // Act
    render(<ShareBar title={TITLE} columns={COLUMNS} items={items} />);

    // Assert
    expect(screen.getByRole("columnheader", { name: "Команда" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Объём" })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Доля" })).toBeInTheDocument();
  });

  it("заголовок — подпись фигуры", () => {
    // Arrange
    const items = makeItems(2);

    // Act
    render(<ShareBar title={TITLE} columns={COLUMNS} items={items} />);

    // Assert
    expect(screen.getByRole("figure", { name: TITLE })).toBeInTheDocument();
  });

  it("доли считаются от суммы и форматируются как проценты", () => {
    // Arrange
    const items = [
      { id: "a", label: "Платформа", value: 3, display: "3 ТБ" },
      { id: "b", label: "Данные", value: 1, display: "1 ТБ" },
    ];

    // Act
    render(<ShareBar title={TITLE} columns={COLUMNS} items={items} />);

    // Assert
    const row = screen.getByRole("row", { name: PLATFORM_ROW });
    expect(within(row).getByText(percent(0.75), RAW)).toBeInTheDocument();
    expect(within(row).getByText("3 ТБ")).toBeInTheDocument();
  });

  it("10 элементов: 8 строк и «Прочие (2)» с суммой остатка", () => {
    // Arrange
    const items = makeItems(10);

    // Act
    render(<ShareBar title={TITLE} columns={COLUMNS} items={items} formatOther={withGb} />);

    // Assert
    const body = screen.getAllByRole("rowgroup")[1];
    expect(within(body as HTMLElement).getAllByRole("row")).toHaveLength(9);
    const other = screen.getByRole("row", { name: OTHER_ROW });
    expect(within(other).getByText("3 ГБ")).toBeInTheDocument();
    expect(within(other).getByText(percent(3 / 55), RAW)).toBeInTheDocument();
  });

  it("8 элементов не сворачиваются", () => {
    // Arrange
    const items = makeItems(8);

    // Act
    render(<ShareBar title={TITLE} columns={COLUMNS} items={items} />);

    // Assert
    expect(screen.queryByText(ANY_OTHER)).toBeNull();
  });

  it("элементы выводятся по убыванию значения", () => {
    // Arrange
    const items = [
      { id: "a", label: "Малая", value: 1, display: "1" },
      { id: "b", label: "Крупная", value: 9, display: "9" },
    ];

    // Act
    render(<ShareBar title={TITLE} columns={COLUMNS} items={items} />);

    // Assert
    const rows = screen.getAllByRole("row");
    expect(rows[1]).toHaveTextContent("Крупная");
  });

  it("нулевая сумма даёт 0 % без NaN", () => {
    // Arrange
    const items = [{ id: "a", label: "Пусто", value: 0, display: "0" }];

    // Act
    render(<ShareBar title={TITLE} columns={COLUMNS} items={items} />);

    // Assert
    expect(screen.getByText(percent(0), RAW)).toBeInTheDocument();
  });

  it("caption выводится под таблицей", () => {
    // Arrange
    const caption = "за 30 дней";

    // Act
    render(<ShareBar title={TITLE} columns={COLUMNS} items={makeItems(2)} caption={caption} />);

    // Assert
    expect(screen.getByText(caption)).toBeInTheDocument();
  });
});
