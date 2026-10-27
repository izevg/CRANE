import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { desktop, screenParameters } from "../frame/screen";
import { Screen } from "./S5Page";

const meta = {
  title: "Экраны/S5 Мои токены",
  component: Screen,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof Screen>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DesktopLight: Story = {
  name: "Desktop 1440 · светлая",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "Мои токены" })).toBeVisible();
    await expect(canvas.getByRole("table", { name: "Токены доступа" })).toBeVisible();
  },
};

export const DesktopDark: Story = {
  name: "Desktop 1440 · тёмная",
  globals: { ...desktop, theme: "dark" },
};

export const Empty: Story = {
  name: "Пустое состояние",
  args: { empty: true },
};

export const English: Story = {
  ...DesktopLight,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "My tokens" })).toBeVisible();
    await expect(canvas.getByRole("navigation", { name: "Primary menu" })).toBeVisible();
  },
};
