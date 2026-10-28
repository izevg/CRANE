import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import { ToggleGroup, ToggleGroupItem } from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/ToggleGroup",
  component: ToggleGroup,
  args: { "aria-label": "Формат пакета", defaultValue: ["npm"] },
  render: (args) => (
    <ToggleGroup {...args}>
      <ToggleGroupItem value="npm">npm</ToggleGroupItem>
      <ToggleGroupItem value="pypi">PyPI</ToggleGroupItem>
      <ToggleGroupItem value="maven">Maven</ToggleGroupItem>
    </ToggleGroup>
  ),
} satisfies Meta<typeof ToggleGroup>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "PyPI" }));
    await expect(canvas.getByRole("button", { name: "PyPI" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await expect(canvas.getByRole("button", { name: "npm" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  },
};

export const Outline: Story = { args: { variant: "outline", size: "sm" } };

export const Multiple: Story = {
  args: { multiple: true, defaultValue: ["npm", "maven"] },
};
