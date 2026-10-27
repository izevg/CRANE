import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
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

export const Deprecated: Story = {
  name: "deprecated: сообщение автора по кнопке",
  args: {
    flags: {
      yanked: false,
      unlisted: false,
      deprecated_message: "Используйте @acme/ui-next",
      dist_tags: [],
      mutable: false,
    },
  },
  play: async ({ canvasElement }) => {
    await userEvent.click(
      within(canvasElement).getByRole("button", { name: "Подробнее: deprecated" }),
    );
    const hint = await within(document.body).findByText("Используйте @acme/ui-next");
    await waitFor(() => expect(hint).toBeVisible());
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
  play: async ({ canvasElement }) => {
    await userEvent.click(
      within(canvasElement).getByRole("button", { name: "Details: deprecated" }),
    );
    const hint = await within(document.body).findByText("Используйте @acme/ui-next");
    await waitFor(() => expect(hint).toBeVisible());
  },
};
