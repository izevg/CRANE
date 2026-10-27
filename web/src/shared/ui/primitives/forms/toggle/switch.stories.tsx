import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import { Switch } from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Switch",
  component: Switch,
  args: { "aria-label": "Показывать устаревшие" },
} satisfies Meta<typeof Switch>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const toggle = within(canvasElement).getByRole("switch", { name: "Показывать устаревшие" });
    await userEvent.click(toggle);
    await expect(toggle).toBeChecked();
  },
};

export const Small: Story = { args: { size: "sm", defaultChecked: true } };

export const Disabled: Story = { args: { disabled: true } };

export const Keyboard: Story = {
  name: "Пробел переключает",
  play: async ({ canvasElement }) => {
    const toggle = within(canvasElement).getByRole("switch");
    await userEvent.tab();
    await userEvent.keyboard(" ");
    await expect(toggle).toBeChecked();
  },
};
