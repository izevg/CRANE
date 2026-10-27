import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { CopyableId } from "~/shared/ui/actions";

const REQUEST_ID = "0192f3a4-7b1c-7e21-9d21-5be0c1d46973";

const meta = {
  title: "Действия/CopyableId",
  component: CopyableId,
  args: { value: REQUEST_ID },
} satisfies Meta<typeof CopyableId>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Truncated: Story = {
  name: "Усечение в середине, полное значение в имени",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("0192…6973")).toBeInTheDocument();
    await expect(canvas.getByRole("button")).toHaveAccessibleName(`Скопировать ID: ${REQUEST_ID}`);
    await expect(canvas.getByTitle(REQUEST_ID)).toBeInTheDocument();
  },
};

export const Full: Story = {
  args: { full: true, label: "Скопировать request_id" },
};

export const Short: Story = {
  args: { value: "1.4.2", label: "Скопировать версию" },
};
