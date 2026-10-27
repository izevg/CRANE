import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { Callout, CalloutDescription, CalloutTitle } from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Callout",
  component: Callout,
  args: { tone: "info" },
} satisfies Meta<typeof Callout>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Tones: Story = {
  render: () => (
    <div className="flex max-w-xl flex-col gap-3">
      <Callout tone="info">
        <CalloutTitle>Ассистенты недоступны</CalloutTitle>
        <CalloutDescription>Поиск работает без семантики.</CalloutDescription>
      </Callout>
      <Callout tone="success">
        <CalloutTitle>Версия 1.4.2 выпущена</CalloutTitle>
      </Callout>
      <Callout tone="warning">
        <CalloutTitle>Upstream npmjs недоступен</CalloutTitle>
        <CalloutDescription>
          Показаны данные от 25 окт (stale-if-error, до 7 дней).
        </CalloutDescription>
      </Callout>
      <Callout tone="error">
        <CalloutTitle>Версия lodash 4.17.20 заблокирована</CalloutTitle>
        <CalloutDescription>
          CVE-2026-31337, ближайшая исправленная версия 4.17.21.
        </CalloutDescription>
      </Callout>
    </div>
  ),
};

export const TonesDark: Story = {
  ...Tones,
  globals: { theme: "dark" },
};

const calloutOf = (canvasElement: HTMLElement, text: string): Element | null =>
  within(canvasElement).getByText(text).closest('[data-slot="callout"]');

export const WithoutRole: Story = {
  name: "По умолчанию без роли",
  args: { children: "Данные обновлены 3 мин назад" },
  play: async ({ canvasElement }) => {
    await expect(calloutOf(canvasElement, "Данные обновлены 3 мин назад")).not.toHaveAttribute(
      "role",
    );
  },
};

export const LiveAlert: Story = {
  args: { tone: "error", live: "alert", children: "Экстренный режим: публикация остановлена" },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("alert")).toHaveTextContent("Экстренный режим");
  },
};

export const LivePolite: Story = {
  args: { tone: "info", live: "polite", children: "Резервная копия запущена" },
  play: async ({ canvasElement }) => {
    await expect(calloutOf(canvasElement, "Резервная копия запущена")).toHaveAttribute(
      "role",
      "status",
    );
  },
};
