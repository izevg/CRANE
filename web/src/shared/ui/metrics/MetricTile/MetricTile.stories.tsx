import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { MetricTile, MetricTileSkeleton } from "~/shared/ui/metrics";

const HIT_RATE = [93.1, 93.8, 94.4, 93.9, 94.6, 95.0, 94.2, 94.8, 94.2];
const QUEUE = [12, 18, 40, 35, 52, 44, 42];
const THREE_MIN_AGO = new Date(Date.now() - 3 * 60 * 1000).toISOString();
const CAPPED_TEXT = /1\s000\+/;
const STALE_TEXT = /обновлено/;

// Пропсы плитки — объединение (тон требует toneLabel), поэтому stories собраны через render, а не args.
const meta = {
  title: "Метрики/MetricTile",
  component: MetricTile,
  decorators: [
    (Story) => (
      <div className="@container/page w-64">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof MetricTile>;

export default meta;
type Story = StoryObj;

export const WithDelta: Story = {
  name: "Объём с дельтой",
  render: () => (
    <MetricTile
      label="Объём хранилищ"
      value={18.4}
      unit="ТБ"
      delta={{ pct: 1.2, polarity: "neutral" }}
    />
  ),
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("Изменение:")).toBeInTheDocument();
  },
};

export const CacheHit: Story = {
  name: "Попадание в кэш со спарклайном",
  decorators: [
    (Story) => (
      <div className="@container/page w-[56rem]">
        <Story />
      </div>
    ),
  ],
  render: () => (
    <div className="w-64">
      <MetricTile label="Попадание в кэш" value={94.2} unit="%" spark={HIT_RATE} reference={94} />
    </div>
  ),
};

export const UpstreamWarning: Story = {
  name: "Upstream 4/5: warning",
  render: () => (
    <MetricTile
      label="Upstream доступны"
      value="4/5"
      tone="warning"
      toneLabel="npmjs недоступен"
      caption="с 13:42"
    />
  ),
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("npmjs недоступен")).toBeInTheDocument();
  },
};

export const Queue: Story = {
  name: "Очередь задач",
  render: () => (
    <MetricTile label="Очередь задач" value={42} caption="старейшая 3 мин" spark={QUEUE} />
  ),
};

export const LastBackup: Story = {
  name: "Последний бэкап",
  render: () => <MetricTile label="Последний бэкап" value="02:14" caption="OK" />,
};

export const P0Open: Story = {
  name: "P0 открыто: error и ссылка",
  parameters: {
    a11y: {
      // Причина: имя ссылки «P0 открыто: 1, требует действия» отличается от видимого текста только знаками
      // препинания (WCAG 2.5.3 это допускает), а axe сравнивает строки дословно (false positive).
      config: { rules: [{ id: "label-content-name-mismatch", enabled: false }] },
    },
  },
  render: () => (
    <MetricTile
      label="P0 открыто"
      value={1}
      tone="error"
      toneLabel="требует действия"
      href="/findings?priority=P0"
      caption="CVE-2026-31337 в @acme/ui"
    />
  ),
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("link", { name: "P0 открыто: 1, требует действия" }),
    ).toHaveAttribute("href", "/findings?priority=P0");
  },
};

export const SlaOverdue: Story = {
  name: "Просрочено SLA",
  render: () => (
    <MetricTile
      label="Просрочено SLA"
      value={3}
      tone="error"
      toneLabel="просрочено"
      delta={{ pct: 50, polarity: "down-good" }}
    />
  ),
};

export const NoData: Story = {
  name: "Нет данных: прочерк, не 0",
  render: () => <MetricTile label="Объём хранилищ" value={null} unit="ТБ" />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Нет данных")).toBeInTheDocument();
    await expect(canvas.queryByText("0")).toBeNull();
  },
};

export const Capped: Story = {
  name: "Усечённый счётчик",
  render: () => <MetricTile label="Находок" value={{ count: 1000, capped: true }} />,
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText(CAPPED_TEXT)).toBeInTheDocument();
  },
};

export const Stale: Story = {
  name: "Ошибка опроса: обновлено 3 мин. назад",
  render: () => (
    <MetricTile
      label="Объём хранилищ"
      value={18.4}
      unit="ТБ"
      stale={{ updatedAt: THREE_MIN_AGO }}
    />
  ),
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText(STALE_TEXT)).toBeInTheDocument();
  },
};

export const TextValue: Story = {
  name: "Текстовое значение",
  render: () => <MetricTile label="Режим" value="норма" />,
};

export const Skeleton: Story = {
  name: "Загрузка",
  render: () => <MetricTileSkeleton />,
};

function Dashboard() {
  return (
    <div className="grid grid-cols-1 gap-4 @3xl/page:grid-cols-3">
      <MetricTile
        label="Объём хранилищ"
        value={18.4}
        unit="ТБ"
        delta={{ pct: 1.2, polarity: "neutral" }}
        spark={QUEUE}
      />
      <MetricTile label="Попадание в кэш" value={94.2} unit="%" spark={HIT_RATE} reference={94} />
      <MetricTile
        label="Upstream доступны"
        value="4/5"
        tone="warning"
        toneLabel="npmjs недоступен"
        spark={QUEUE}
      />
    </div>
  );
}

export const GridNarrow: Story = {
  name: "Сетка: узкий контейнер",
  decorators: [
    (Story) => (
      <div className="@container/page w-[22rem]">
        <Story />
      </div>
    ),
  ],
  render: () => <Dashboard />,
};

export const GridWide: Story = {
  name: "Сетка: широкий контейнер",
  decorators: [
    (Story) => (
      <div className="@container/page w-[60rem]">
        <Story />
      </div>
    ),
  ],
  render: () => <Dashboard />,
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  decorators: GridWide.decorators ?? [],
  render: () => <Dashboard />,
};
