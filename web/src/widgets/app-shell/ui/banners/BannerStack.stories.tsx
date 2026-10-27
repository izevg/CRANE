import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { type Banner, BannerStack } from "./BannerStack";

const MORE = /Ещё/;
const COLLAPSE = /Свернуть/;

const READ_ONLY: Banner = {
  id: "read-only",
  tone: "warning",
  text: "Только чтение: основная БД недоступна, публикация временно отключена. Повторите через 2 мин",
  action: { label: "Статус платформы", run: { href: "/platform" } },
};
const SOD: Banner = {
  id: "sod",
  tone: "warning",
  text: "Включён режим разделения обязанностей: действия требуют второго подтверждения",
};
const UPSTREAM: Banner = {
  id: "upstream",
  tone: "warning",
  text: "Upstream npmjs недоступен: показаны данные от 25 окт (stale-if-error, до 7 дней)",
};
const LLM: Banner = {
  id: "llm",
  tone: "info",
  text: "Ассистенты недоступны, поиск работает без семантики",
};
const P0: Banner = {
  id: "p0",
  tone: "error",
  urgent: true,
  text: "P0 без действия более 1 часа: CVE-2026-31337 в @acme/ui 1.4.2",
  action: { label: "Открыть находку", run: { href: "/findings/CVE-2026-31337" } },
};

const meta = {
  title: "Шелл/BannerStack",
  component: BannerStack,
  args: { banners: [READ_ONLY], onCollapse: fn() },
} satisfies Meta<typeof BannerStack>;

export default meta;
type Story = StoryObj<typeof meta>;

export const One: Story = {
  name: "Один баннер",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText(READ_ONLY.text)).toBeVisible();
    await expect(canvas.queryByRole("button", { name: MORE })).toBeNull();
    await expect(canvas.queryByRole("alert")).toBeNull();
  },
};

export const Two: Story = {
  name: "Два баннера",
  args: { banners: [READ_ONLY, LLM] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText(LLM.text)).toBeVisible();
    await expect(canvas.getByRole("button", { name: COLLAPSE })).toBeVisible();
  },
};

export const FourWithMore: Story = {
  name: "Четыре баннера: два + «Ещё 2»",
  args: { banners: [READ_ONLY, SOD, UPSTREAM, LLM] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByText(UPSTREAM.text)).toBeNull();
    const more = canvas.getByRole("button", { name: "Ещё 2" });
    await expect(more).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(more);
    await expect(canvas.getByText(UPSTREAM.text)).toBeVisible();
    await expect(canvas.getByText(LLM.text)).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Скрыть остальные" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  },
};

function centerY(rect: DOMRect): number {
  return rect.top + rect.height / 2;
}

/** Центр первой строки текста: Range по первому текстовому узлу даёт прямоугольник строки, а не блока. */
function firstLineCenter(element: Element): number {
  const range = document.createRange();
  range.selectNodeContents(element);
  const first = range.getClientRects()[0];
  if (first === undefined) {
    throw new Error("у текста нет строк");
  }
  return centerY(first);
}

const ICON_TOLERANCE = 2;
const ICON_SIZE = 20;

export const IconAlignment: Story = {
  name: "Иконка 20 px по центру первой строки (одна и несколько строк, с действием)",
  args: {
    banners: [{ ...READ_ONLY, action: { label: "Статус платформы", run: { href: "/platform" } } }],
  },
  decorators: [
    (Story) => (
      <div className="w-[26rem]">
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    const callout = canvasElement.querySelector("[data-slot=callout]");
    const icon = callout?.querySelector("svg");
    if (!(callout && icon)) {
      throw new Error("нет баннера или иконки");
    }
    const text = within(callout as HTMLElement).getByText(READ_ONLY.text);
    await expect(icon.getBoundingClientRect().width).toBeCloseTo(ICON_SIZE, 0);
    // текст на 26rem переносится на несколько строк: иконка выровнена по первой
    await expect(text.getClientRects().length).toBeGreaterThan(0);
    await expect(
      Math.abs(centerY(icon.getBoundingClientRect()) - firstLineCenter(text)),
    ).toBeLessThanOrEqual(ICON_TOLERANCE);
    const action = within(callout as HTMLElement).getByRole("link", { name: "Статус платформы" });
    await expect(
      Math.abs(centerY(action.getBoundingClientRect()) - firstLineCenter(text)),
    ).toBeLessThanOrEqual(ICON_TOLERANCE);
  },
};

export const IconAlignmentSingleLine: Story = {
  name: "Иконка по центру однострочного баннера",
  args: { banners: [LLM] },
  play: async ({ canvasElement }) => {
    const callout = canvasElement.querySelector("[data-slot=callout]");
    const icon = callout?.querySelector("svg");
    if (!(callout && icon)) {
      throw new Error("нет баннера или иконки");
    }
    const text = within(callout as HTMLElement).getByText(LLM.text);
    await expect(
      Math.abs(centerY(icon.getBoundingClientRect()) - firstLineCenter(text)),
    ).toBeLessThanOrEqual(ICON_TOLERANCE);
    await expect(
      Math.abs(centerY(icon.getBoundingClientRect()) - centerY(callout.getBoundingClientRect())),
    ).toBeLessThanOrEqual(ICON_TOLERANCE);
  },
};

export const UrgentP0: Story = {
  name: "Срочный P0: role=alert",
  args: { banners: [P0, READ_ONLY] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("alert")).toHaveTextContent(P0.text);
    await expect(canvas.getByRole("link", { name: "Открыть находку" })).toBeVisible();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: { banners: [P0, READ_ONLY, LLM] },
};

export const Empty: Story = {
  name: "Нет баннеров — зона не рисуется",
  args: { banners: [] },
  play: async ({ canvasElement }) => {
    await expect(canvasElement.querySelector("[data-slot=banner-stack]")).toBeNull();
  },
};
