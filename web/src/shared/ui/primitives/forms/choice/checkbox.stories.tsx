import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import { Checkbox } from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Checkbox",
  component: Checkbox,
  args: { "aria-label": "Выбрать версию 1.4.2" },
} satisfies Meta<typeof Checkbox>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const box = within(canvasElement).getByRole("checkbox", { name: "Выбрать версию 1.4.2" });
    await userEvent.click(box);
    await expect(box).toBeChecked();
  },
};

export const Indeterminate: Story = {
  args: { indeterminate: true, "aria-label": "Выбрать все версии" },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("checkbox")).toBePartiallyChecked();
  },
};

export const Disabled: Story = { args: { disabled: true, checked: true } };
