import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { RelativeTime } from "~/shared/ui/status";

const MINUTE = 60_000;
const MINUTES_AGO = /мин\. назад/;

const meta = {
  title: "Статус/RelativeTime",
  component: RelativeTime,
  args: { ts: new Date(Date.now() - 12 * MINUTE) },
} satisfies Meta<typeof RelativeTime>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Minutes: Story = {
  play: async ({ canvasElement }) => {
    const time = within(canvasElement).getByText(MINUTES_AGO);
    await expect(time.tagName).toBe("TIME");
    await expect(time).toHaveAttribute("datetime");
    await expect(time).toHaveAttribute("title");
  },
};

export const Old: Story = {
  name: "Старше суток — дата",
  args: { ts: "2025-03-12T09:30:00Z" },
};

export const Invalid: Story = {
  name: "Некорректная дата — прочерк",
  args: { ts: "не дата" },
};
