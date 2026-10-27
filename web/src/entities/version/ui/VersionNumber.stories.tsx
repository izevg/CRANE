import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import { VersionNumber } from "../index";

const meta = {
  title: "Сущности/Версия/Номер",
  component: VersionNumber,
  args: { name: "1.4.1", state: "released" },
} satisfies Meta<typeof VersionNumber>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Regular: Story = {
  name: "Обычная версия: моно",
  play: async ({ canvasElement }) => {
    const version = within(canvasElement).getByText("1.4.1");
    await expect(getComputedStyle(version).fontFamily).toContain("JetBrains Mono");
  },
};

/** Остаётся открытым в момент axe-проверки. */
export const Deleted: Story = {
  name: "Удалённая: зачёркнут, подсказка про повторное использование",
  args: { name: "1.3.9", state: "deleted" },
  play: async ({ canvasElement }) => {
    const version = within(canvasElement).getByText("1.3.9");
    await expect(getComputedStyle(version).textDecorationLine).toBe("line-through");
    await userEvent.tab();
    await expect(
      await within(document.body).findAllByText("Номер версии нельзя использовать повторно"),
    ).not.toHaveLength(0);
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: { name: "1.3.9", state: "deleted" },
};

export const English: Story = {
  name: "EN: подсказка",
  globals: { locale: "en" },
  args: { name: "1.3.9", state: "deleted" },
  play: async () => {
    await userEvent.tab();
    await expect(
      await within(document.body).findAllByText("Version number can't be reused"),
    ).not.toHaveLength(0);
  },
};
