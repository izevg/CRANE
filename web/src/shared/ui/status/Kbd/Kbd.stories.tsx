import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { Kbd } from "~/shared/ui/status";

const meta = {
  title: "Статус/Kbd",
  component: Kbd,
  args: { children: "Ctrl" },
} satisfies Meta<typeof Kbd>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Shortcut: Story = {
  render: () => (
    <span className="inline-flex items-center gap-1 text-body">
      Поиск <Kbd>Ctrl</Kbd>
      <Kbd>K</Kbd>
    </span>
  ),
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("K").tagName).toBe("KBD");
  },
};
