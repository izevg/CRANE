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

const MIN_CONTRAST = 4.5;
const CHANNELS = /\d+(\.\d+)?/g;

function luminance(color: string): number {
  const [r = 0, g = 0, b = 0] = (color.match(CHANNELS) ?? []).slice(0, 3).map(Number);
  const lin = (value: number) => {
    const v = value / 255;
    return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * lin(r) + 0.7152 * lin(g) + 0.0722 * lin(b);
}

function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return ((hi ?? 0) + 0.05) / ((lo ?? 0) + 0.05);
}

/** Подсказка на токенах popover: фон равен `--popover`, текст читаем (≥ 4,5:1), дата короткая. */
async function expectTokenTooltip(canvasElement: HTMLElement) {
  await waitFor(() => expect(canvasElement.querySelector("svg.recharts-surface")).not.toBeNull(), {
    timeout: LOAD_TIMEOUT,
  });
  const surface = canvasElement.querySelector(".recharts-wrapper") as HTMLElement;
  const box = surface.getBoundingClientRect();
  await userEvent.pointer({
    target: surface,
    coords: { clientX: box.left + box.width / 2, clientY: box.top + box.height / 2 },
  });
  const tooltip = (await waitFor(() => {
    const found = canvasElement.querySelector(".recharts-default-tooltip") as HTMLElement | null;
    if (!found) {
      throw new Error("tooltip not shown");
    }
    return found;
  })) as HTMLElement;
  const probe = document.createElement("div");
  probe.style.backgroundColor = "var(--popover)";
  document.body.append(probe);
  const popover = getComputedStyle(probe).backgroundColor;
  probe.remove();
  const style = getComputedStyle(tooltip);
  await expect(style.backgroundColor).toBe(popover);
  await expect(contrast(style.color, style.backgroundColor)).toBeGreaterThanOrEqual(MIN_CONTRAST);
  const label = tooltip.querySelector(".recharts-tooltip-label") as HTMLElement;
  await expect(contrast(getComputedStyle(label).color, popover)).toBeGreaterThanOrEqual(
    MIN_CONTRAST,
  );
  await expect(label.textContent).not.toContain("2026 г.");
}

export const TooltipLight: Story = {
  name: "Подсказка на токенах popover",
  play: async ({ canvasElement }) => {
    await expectTokenTooltip(canvasElement);
  },
};

export const TooltipDark: Story = {
  name: "Подсказка на токенах popover, тёмная тема",
  globals: { theme: "dark" },
  play: async ({ canvasElement }) => {
    await expectTokenTooltip(canvasElement);
  },
};

export const English: Story = {
  name: "EN: подписи на английском",
  globals: { locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Show as table" }));
    await expect(await canvas.findByRole("columnheader", { name: "Time" })).toBeInTheDocument();
    await expect(canvas.getByRole("button", { name: "Show as chart" })).toBeInTheDocument();
  },
};
