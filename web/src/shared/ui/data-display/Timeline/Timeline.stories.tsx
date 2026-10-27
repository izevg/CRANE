import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { Timeline } from "~/shared/ui/data-display";

const meta = {
  title: "Данные/Timeline",
  component: Timeline,
  args: { label: "Жизненный цикл версии 1.4.2", items: [] },
} satisfies Meta<typeof Timeline>;

export default meta;
type Story = StoryObj<typeof meta>;

export const VersionLifecycle: Story = {
  name: "Жизненный цикл версии",
  args: {
    items: [
      {
        id: "published",
        time: "27.10.2026, 13:42",
        title: "Опубликована",
        caption: "robot:ci-publisher",
      },
      {
        id: "sha",
        time: "27.10.2026, 13:42",
        title: "SHA-256 подтверждён",
        caption: "a88d…6973",
      },
      {
        id: "scan",
        time: "27.10.2026, 13:48",
        title: "Скан найден: 1 уязвимость",
        tone: "warning",
        tag: "Карантин",
        link: { label: "CVE-2026-31337", run: { href: "/findings/CVE-2026-31337" } },
      },
      {
        id: "gates",
        time: "27.10.2026, 13:49",
        title: "Гейт безопасности: отказ",
        tone: "error",
        tag: "Заблокирована",
      },
      {
        id: "request",
        time: "27.10.2026, 14:07",
        title: "Запрос на продвижение в release",
        tone: "info",
        tag: "ожидает AppSec",
      },
    ],
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("list", { name: "Жизненный цикл версии 1.4.2" })).toBeVisible();
    await expect(canvas.getAllByRole("listitem")).toHaveLength(5);
    await expect(canvas.getByText("Заблокирована")).toBeVisible();
  },
};

export const PriorityHistory: Story = {
  name: "История приоритета",
  args: {
    label: "История приоритета находки",
    items: [
      {
        id: "p1",
        time: "27.10.2026, 14:07",
        title: "Приоритет повышен до P1",
        tone: "error",
        tag: "по правилу",
        caption: "Эксплуатируется: CISA Known Exploited",
      },
      {
        id: "p2",
        time: "27.10.2026, 13:55",
        title: "Оценка: P2",
        tone: "warning",
        tag: "оценка модели",
      },
      { id: "p3", time: "27.10.2026, 13:48", title: "Находка создана", tag: "фид" },
    ],
  },
};

export const Dark: Story = {
  ...VersionLifecycle,
  globals: { theme: "dark" },
};
