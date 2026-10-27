import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { AutoRefreshToggle } from "~/shared/ui/actions";

const meta = {
  title: "Действия/AutoRefreshToggle",
  component: AutoRefreshToggle,
  args: { paused: false, onPausedChange: fn() },
} satisfies Meta<typeof AutoRefreshToggle>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Active: Story = {
  name: "Автообновление идёт",
  play: async ({ canvasElement, args }) => {
    const button = within(canvasElement).getByRole("button", { name: "Пауза автообновления" });
    await expect(button).toHaveAttribute("aria-pressed", "false");
    await userEvent.click(button);
    await expect(args.onPausedChange).toHaveBeenCalledWith(true);
  },
};

export const Paused: Story = {
  name: "На паузе",
  args: { paused: true },
  play: async ({ canvasElement, args }) => {
    const button = within(canvasElement).getByRole("button", { name: "Пауза автообновления" });
    await expect(button).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(button);
    await expect(args.onPausedChange).toHaveBeenCalledWith(false);
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: { paused: true },
};
