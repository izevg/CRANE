import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { EmptyState } from "~/shared/ui/status";

const onPublish = fn();
const onReset = fn();

const meta = {
  title: "Статус/EmptyState",
  component: EmptyState,
} satisfies Meta<typeof EmptyState>;

export default meta;
type Story = StoryObj<typeof meta>;

export const FirstRun: Story = {
  name: "Первый запуск: primary и secondary",
  args: {
    variant: "first-run",
    title: "В репозитории пока нет пакетов",
    description: "Опубликуйте первый пакет из CI или импортируйте существующие из другого реестра.",
    primary: { label: "Как опубликовать", run: { onSelect: onPublish } },
    secondary: { label: "Импорт из Nexus", run: { href: "/import/nexus" } },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Как опубликовать" }));
    await expect(onPublish).toHaveBeenCalledTimes(1);
    await expect(canvas.getByRole("link", { name: "Импорт из Nexus" })).toHaveAttribute(
      "href",
      "/import/nexus",
    );
  },
};

export const FirstRunWithoutActions: Story = {
  name: "Первый запуск: без действий",
  args: {
    variant: "first-run",
    title: "Нет токенов доступа",
    description: "Токены, выпущенные для CI, появятся здесь.",
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.queryByRole("button")).toBeNull();
    await expect(canvas.queryByRole("link")).toBeNull();
  },
};

export const Filtered: Story = {
  name: "Фильтры ничего не нашли",
  args: {
    variant: "filtered",
    reset: { label: "Сбросить фильтры", run: { onSelect: onReset } },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Сбросить фильтры" }));
    await expect(onReset).toHaveBeenCalledTimes(1);
  },
};

export const InPanel: Story = {
  name: "В панели: заголовок третьего уровня",
  args: {
    variant: "filtered",
    title: "Находок с такими фильтрами нет",
    headingLevel: 3,
  },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("heading", {
        level: 3,
        name: "Находок с такими фильтрами нет",
      }),
    ).toBeVisible();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: {
    variant: "first-run",
    title: "В репозитории пока нет пакетов",
    description: "Опубликуйте первый пакет из CI.",
    primary: { label: "Как опубликовать", run: { onSelect: onPublish } },
  },
};
