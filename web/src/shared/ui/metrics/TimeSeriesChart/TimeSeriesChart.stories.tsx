import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
import { TimeSeriesChart } from "~/shared/ui/metrics";

const LOAD_TIMEOUT = 8000;
const START = Date.UTC(2026, 9, 26, 12, 0, 0);
const STEP = 60 * 60 * 1000;
const HOURS = 24;

function series(base: number, amp: number) {
  return Array.from({ length: HOURS }, (_, index) => ({
    t: START + index * STEP,
    v: Math.round(base + amp * Math.sin(index / 3) + (index % 5)),
  }));
}

const meta = {
  title: "Метрики/TimeSeriesChart",
  component: TimeSeriesChart,
  args: {
    title: "Задержки запросов",
    summary: "p95 за 24 ч: 84 мс, максимум 138 мс в 21:40, SLO 150 мс не нарушен",
    unit: "мс",
    series: [
      { id: "p50", label: "p50", points: series(30, 8) },
      { id: "p95", label: "p95", points: series(84, 20) },
      { id: "p99", label: "p99", dash: true, points: series(120, 24) },
    ],
    slo: { value: 150, label: "SLO 150 мс" },
  },
  decorators: [
    (Story) => (
      <div className="w-[48rem]">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof TimeSeriesChart>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Percentiles: Story = {
  name: "p50/p95/p99 с SLO, штрих у p99",
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("figure", { name: "Задержки запросов" }),
    ).toBeInTheDocument();
    await waitFor(
      () => expect(canvasElement.querySelector("svg.recharts-surface")).not.toBeNull(),
      { timeout: LOAD_TIMEOUT },
    );
  },
};

export const SingleSeriesArea: Story = {
  name: "Один ряд с заливкой",
  render: () => (
    <TimeSeriesChart
      title="Запросов в минуту"
      summary="Пик 4 210 запросов в минуту в 14:20"
      unit="зап/мин"
      area
      series={[{ id: "rpm", label: "Запросы", points: series(3000, 900) }]}
    />
  ),
  play: async ({ canvasElement }) => {
    await waitFor(
      () => expect(canvasElement.querySelector("svg.recharts-surface")).not.toBeNull(),
      { timeout: LOAD_TIMEOUT },
    );
  },
};

export const AsTable: Story = {
  name: "Показать таблицей",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Показать таблицей" }));
    await expect(await canvas.findByRole("table")).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Показать графиком" })).toBeInTheDocument();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
};
