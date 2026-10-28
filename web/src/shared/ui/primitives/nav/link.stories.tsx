import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { Link } from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Link",
  component: Link,
  args: { href: "/packages/acme-ui/versions/1.4.2", children: "Версия 1.4.2" },
} satisfies Meta<typeof Link>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  name: "Через LinkProvider витрины",
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("link", { name: "Версия 1.4.2" })).toHaveAttribute(
      "href",
      "/packages/acme-ui/versions/1.4.2",
    );
  },
};
