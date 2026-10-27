import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { desktop, screenParameters } from "../frame/screen";
import { STALE_TEXT, TB_EN } from "./S6Data";
import { S6Platform } from "./S6Page";

const meta = {
  title: "Экраны/S6 Обзор платформы",
  component: S6Platform,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof S6Platform>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Desktop1440: Story = {
  name: "Desktop 1440, светлая",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "Платформа" })).toBeInTheDocument();
    await expect(
      canvas.getByRole("heading", { level: 2, name: "Состояние компонентов" }),
    ).toBeInTheDocument();
  },
};

export const DesktopDark: Story = {
  name: "Desktop 1440, тёмная",
  globals: { ...desktop, theme: "dark" },
};

export const TileDataStates: Story = {
  name: "Плитки: нет данных и ошибка опроса",
  args: { mode: "degraded" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Нет данных")).toBeInTheDocument();
    await expect(canvas.getByText(STALE_TEXT)).toBeInTheDocument();
  },
};

export const English: Story = {
  ...Desktop1440,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "Platform" })).toBeVisible();
    await expect(canvas.getByText("Quick actions")).toBeVisible();
    // Размеры в легенде — в формате английской локали: «5.1 TB», а не «5,1 ТБ»
    await expect(canvas.getAllByText(TB_EN).length).toBeGreaterThan(0);
  },
};
