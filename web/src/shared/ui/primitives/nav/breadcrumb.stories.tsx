import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Breadcrumb",
  component: Breadcrumb,
  render: () => (
    <Breadcrumb>
      <BreadcrumbList>
        <BreadcrumbItem>
          <BreadcrumbLink href="/packages">Пакеты</BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
          <BreadcrumbLink href="/packages/acme-ui">@acme/ui</BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
          <BreadcrumbPage>1.4.2</BreadcrumbPage>
        </BreadcrumbItem>
      </BreadcrumbList>
    </Breadcrumb>
  ),
} satisfies Meta<typeof Breadcrumb>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      canvas.getByRole("navigation", { name: "Навигационная цепочка" }),
    ).toBeInTheDocument();
    await expect(canvas.getByText("1.4.2")).toHaveAttribute("aria-current", "page");
    await expect(canvas.getByRole("link", { name: "@acme/ui" })).toHaveAttribute(
      "href",
      "/packages/acme-ui",
    );
  },
};
