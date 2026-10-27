import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import { VersionFlags } from "../index";

const meta = {
  title: "Сущности/Версия/Флаги",
  component: VersionFlags,
  args: {
    flags: {
      yanked: false,
      unlisted: false,
      deprecated_message: null,
      dist_tags: ["latest"],
      mutable: false,
    },
  },
} satisfies Meta<typeof VersionFlags>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DistTag: Story = {
  name: "dist-tag",
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("latest")).toBeVisible();
  },
};

export const All: Story = {
  name: "Все флаги",
  args: {
    flags: {
      yanked: true,
      unlisted: true,
      deprecated_message: null,
      dist_tags: ["latest", "next"],
      mutable: true,
    },
  },
};

/** Остаётся открытым в момент axe-проверки. */
export const Deprecated: Story = {
  name: "deprecated: сообщение автора в подсказке",
  args: {
    flags: {
      yanked: false,
      unlisted: false,
      deprecated_message: "Используйте @acme/ui-next",
      dist_tags: [],
      mutable: false,
    },
  },
  play: async () => {
    await userEvent.tab();
    await expect(
      await within(document.body).findAllByText("Используйте @acme/ui-next"),
    ).not.toHaveLength(0);
  },
};

export const Empty: Story = {
  name: "Нет флагов — ничего не рисуется",
  args: {
    flags: {
      yanked: false,
      unlisted: false,
      deprecated_message: null,
      dist_tags: [],
      mutable: false,
    },
  },
  play: async ({ canvasElement }) => {
    await expect(canvasElement.querySelector("[data-slot=badge]")).toBeNull();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: { ...All.args },
};

export const English: Story = {
  name: "EN: подсказка остаётся текстом автора",
  globals: { locale: "en" },
  args: { ...Deprecated.args },
  play: async () => {
    await userEvent.tab();
    await expect(
      await within(document.body).findAllByText("Используйте @acme/ui-next"),
    ).not.toHaveLength(0);
  },
};
