import type { Meta, StoryObj } from "@storybook/react-vite";
import { Skeleton } from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Skeleton",
  component: Skeleton,
} satisfies Meta<typeof Skeleton>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Row: Story = {
  render: () => (
    <div className="flex max-w-md flex-col gap-2" role="status" aria-label="Загрузка версий">
      <Skeleton className="h-5 w-48" />
      <Skeleton className="h-4 w-full" />
      <Skeleton className="h-4 w-2/3" />
    </div>
  ),
};
