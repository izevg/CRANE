import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { CopyableId } from "~/shared/ui/actions";
import { Timeline } from "~/shared/ui/data-display";

const SHA = "a88d4f0e91c7b2236f5d0c8a7e4b19d3c5f6a2810b7e94d3c1f58a2b6e1d6973";

const meta = {
  title: "Данные/Timeline",
  component: Timeline,
  args: { label: "Жизненный цикл версии 1.4.2", items: [] },
} satisfies Meta<typeof Timeline>;

export default meta;
type Story = StoryObj<typeof meta>;

const LIFECYCLE = [
  {
    id: "published",
    time: "2026-10-27T13:42:00Z",
    title: "Опубликована",
    actor: "crr_9f8e7d6c · ui-release",
    source: "pipeline #4521",
  },
  {
    id: "sha",
    time: "2026-10-27T13:42:10Z",
    title: "SHA-256 подтверждён",
    detail: <CopyableId value={SHA} label="Скопировать SHA-256" />,
  },
  {
    id: "scan",
    time: "2026-10-27T13:48:00Z",
    title: "Скан: найдена 1 уязвимость в lodash 4.17.20",
    tone: "warning",
    status: { tone: "warning", label: "Карантин" },
    source: "Trivy",
    link: { label: "CVE-2026-31337", run: { href: "/findings/CVE-2026-31337" } },
  },
  {
    id: "gates",
    time: "2026-10-27T13:49:00Z",
    title: "Гейт безопасности: отказ",
    tone: "error",
    status: { tone: "error", label: "Не пройден" },
  },
  {
    id: "request",
    time: "2026-10-27T14:07:00Z",
    title: "Запрос на продвижение в release",
    tone: "info",
    status: { tone: "info", label: "Ждёт AppSec" },
    actor: "Иванов И.",
  },
] as const;

const PRIORITY = [
  {
    id: "p1",
    time: "2026-10-27T14:07:00Z",
    title: "Приоритет повышен до P1",
    tone: "error",
    status: { tone: "error", label: "P1" },
    source: "по правилу",
    detail: "Эксплуатируется: CISA Known Exploited",
  },
  {
    id: "p2",
    time: "2026-10-27T13:55:00Z",
    title: "Оценка модели: P2",
    tone: "warning",
    status: { tone: "warning", label: "P2" },
    source: "оценка модели",
  },
  {
    id: "p3",
    time: "2026-10-27T13:48:00Z",
    title: "Находка создана",
    source: "фид",
    actor: "robot:feed-ingest",
  },
] as const;

const FONT_PROBE = "x";

/** Базовая линия первой строки текста элемента: низ строки минус спуск шрифта. */
function baselineOf(element: Element): number {
  const walker = document.createTreeWalker(element, NodeFilter.SHOW_TEXT);
  let text = walker.nextNode();
  while (text && !text.textContent?.trim()) {
    text = walker.nextNode();
  }
  const range = document.createRange();
  range.selectNodeContents(text as Node);
  const rect = range.getClientRects()[0] as DOMRect;
  const style = getComputedStyle((text as Node).parentElement as Element);
  const context = document.createElement("canvas").getContext("2d") as CanvasRenderingContext2D;
  context.font = `${style.fontWeight} ${style.fontSize} ${style.fontFamily}`;
  return rect.bottom - context.measureText(FONT_PROBE).fontBoundingBoxDescent;
}

async function expectAnatomy(canvasElement: HTMLElement) {
  const canvas = within(canvasElement);
  const title = canvas.getByText("Скан: найдена 1 уязвимость в lodash 4.17.20");
  const link = canvas.getByRole("link", { name: "CVE-2026-31337" });
  // Левый край ссылки совпадает с левым краем заголовка (±1 px)
  await expect(
    Math.abs(link.getBoundingClientRect().left - title.getBoundingClientRect().left),
  ).toBeLessThanOrEqual(1);
  // Чип статуса на одной базовой линии с заголовком (±2 px)
  const chip = canvas.getByText("Карантин");
  await expect(Math.abs(baselineOf(chip) - baselineOf(title))).toBeLessThanOrEqual(2);
}

export const VersionLifecycle: Story = {
  name: "Жизненный цикл версии",
  args: { items: LIFECYCLE },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("list", { name: "Жизненный цикл версии 1.4.2" })).toBeVisible();
    await expect(canvas.getAllByRole("listitem")).toHaveLength(5);
    await expectAnatomy(canvasElement);
  },
};

export const PriorityHistory: Story = {
  name: "История приоритета",
  args: { label: "История приоритета находки", items: PRIORITY },
};

export const Narrow: Story = {
  name: "Узкая лента 280 px: статус под заголовком",
  args: { items: LIFECYCLE },
  decorators: [
    (Story) => (
      <div className="w-[280px]">
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const title = canvas.getByText("Скан: найдена 1 уязвимость в lodash 4.17.20");
    const chip = canvas.getByText("Карантин");
    await expect(chip.getBoundingClientRect().top).toBeGreaterThanOrEqual(
      title.getBoundingClientRect().bottom - 1,
    );
    const list = canvas.getByRole("list", { name: "Жизненный цикл версии 1.4.2" });
    await expect(list.scrollWidth).toBeLessThanOrEqual(list.clientWidth);
    // Мета, деталь и ссылка начинаются от левого края заголовка, а не от колонки точки (±1 px)
    const left = title.getBoundingClientRect().left;
    const meta = (canvas.getAllByText("Trivy")[0] as HTMLElement).closest("div") as HTMLElement;
    const link = canvas.getByRole("link", { name: "CVE-2026-31337" });
    await expect(Math.abs(meta.getBoundingClientRect().left - left)).toBeLessThanOrEqual(1);
    await expect(Math.abs(link.getBoundingClientRect().left - left)).toBeLessThanOrEqual(1);
  },
};

export const InPopoverDark: Story = {
  name: "Тёмная тема в Popover: точка вырезана цветом popover",
  args: { items: LIFECYCLE },
  globals: { theme: "dark" },
  decorators: [
    (Story) => (
      <div
        className="w-[360px] rounded-xl bg-popover p-4 text-popover-foreground"
        style={{ "--timeline-surface": "var(--popover)" } as React.CSSProperties}
      >
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    const dot = canvasElement.querySelector('[data-slot="timeline-dot"]') as HTMLElement;
    const probe = document.createElement("div");
    probe.style.backgroundColor = "var(--popover)";
    document.body.append(probe);
    const popover = getComputedStyle(probe).backgroundColor;
    probe.remove();
    await expect(getComputedStyle(dot).boxShadow).toContain(popover);
  },
};
