// biome-ignore-all lint/style/noMagicNumbers: тестовые временные ряды — произвольные числа
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { TimeSeriesChart } from "~/shared/ui/metrics";

const TITLE = "Запросы и задержки";
const SUMMARY = "p95 за 24 ч: 84 мс, максимум 138 мс в 21:40, SLO 150 мс не нарушен";
const SHOW_TABLE = "Показать таблицей";
const SHOW_CHART = "Показать графиком";
const T0 = Date.UTC(2026, 9, 27, 12, 0, 0);
const MINUTE = 60_000;
const SKELETON = "[data-slot=skeleton]";
const PLOT = "[data-slot=time-series-plot]";

const P95 = {
  id: "p95",
  label: "p95",
  points: [
    { t: T0, v: 84 },
    { t: T0 + MINUTE, v: 138 },
  ],
};
const P99 = {
  id: "p99",
  label: "p99",
  dash: true,
  points: [{ t: T0 + MINUTE, v: 210 }],
};

describe("TimeSeriesChart", () => {
  it("это figure с заголовком в figcaption и текстовой сводкой", () => {
    // Arrange
    const series = [P95];

    // Act
    render(<TimeSeriesChart title={TITLE} summary={SUMMARY} series={series} />);

    // Assert
    const figure = screen.getByRole("figure", { name: TITLE });
    expect(within(figure).getByText(TITLE).tagName).toBe("FIGCAPTION");
    expect(within(figure).getByText(SUMMARY)).toBeInTheDocument();
  });

  it("«Показать таблицей» показывает таблицу с теми же значениями", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<TimeSeriesChart title={TITLE} summary={SUMMARY} series={[P95]} unit="мс" />);

    // Act
    await user.click(screen.getByRole("button", { name: SHOW_TABLE }));

    // Assert
    const table = await screen.findByRole("table");
    expect(within(table).getByRole("columnheader", { name: "p95" })).toBeInTheDocument();
    expect(within(table).getByText("84 мс")).toBeInTheDocument();
    expect(within(table).getByText("138 мс")).toBeInTheDocument();
  });

  it("в таблице отсутствующее значение ряда показывается прочерком", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<TimeSeriesChart title={TITLE} summary={SUMMARY} series={[P95, P99]} />);

    // Act
    await user.click(screen.getByRole("button", { name: SHOW_TABLE }));

    // Assert
    const table = await screen.findByRole("table");
    expect(within(table).getAllByRole("row")).toHaveLength(3);
    expect(within(table).getByText("—")).toBeInTheDocument();
    expect(within(table).getByText("210")).toBeInTheDocument();
  });

  it("кнопка переключает подпись и возвращает график", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<TimeSeriesChart title={TITLE} summary={SUMMARY} series={[P95]} />);
    await user.click(screen.getByRole("button", { name: SHOW_TABLE }));

    // Act
    await user.click(screen.getByRole("button", { name: SHOW_CHART }));

    // Assert
    expect(screen.queryByRole("table")).toBeNull();
    expect(screen.getByRole("button", { name: SHOW_TABLE })).toBeInTheDocument();
  });

  it("лениво загружаемый график заменяет заглушку, сводка остаётся на месте", async () => {
    // Arrange
    const { container } = render(
      <TimeSeriesChart title={TITLE} summary={SUMMARY} series={[P95]} />,
    );

    // Act
    await waitFor(() => expect(container.querySelector(SKELETON)).toBeNull());

    // Assert
    expect(screen.getByText(SUMMARY)).toBeInTheDocument();
    expect(container.querySelector(PLOT)).not.toBeNull();
  });
});
