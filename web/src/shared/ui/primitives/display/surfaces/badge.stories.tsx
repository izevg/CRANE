import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { Badge } from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Badge",
  component: Badge,
  args: { children: "npm" },
} satisfies Meta<typeof Badge>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Format: Story = {
  args: { variant: "secondary" },
};

export const Variants: Story = {
  render: () => (
    <div className="flex flex-wrap items-center gap-2">
      <Badge variant="default">MFA</Badge>
      <Badge variant="secondary">npm</Badge>
      <Badge variant="outline">yanked</Badge>
      <Badge variant="outline">unlisted</Badge>
      <Badge variant="destructive">shadow</Badge>
    </div>
  ),
};

export const AsLink: Story = {
  name: "Как ссылка (render)",
  render: () => (
    <Badge variant="outline" render={<a href="/packages/acme-ui?tag=latest" />}>
      latest
    </Badge>
  ),
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("link", { name: "latest" })).toHaveAttribute(
      "href",
      "/packages/acme-ui?tag=latest",
    );
  },
};

export const Dark: Story = {
  ...Variants,
  globals: { theme: "dark" },
};
