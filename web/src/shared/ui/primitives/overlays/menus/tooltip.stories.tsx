import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
import { Button, Tooltip, TooltipContent, TooltipTrigger } from "~/shared/ui/primitives";

function BlockedHint({ defaultOpen = false }: { defaultOpen?: boolean }) {
  return (
    <Tooltip defaultOpen={defaultOpen}>
      <TooltipTrigger render={<Button variant="secondary" />}>Снять блок</TooltipTrigger>
      <TooltipContent>Нужна роль AppSec</TooltipContent>
    </Tooltip>
  );
}

const meta = {
  title: "Примитивы/Tooltip",
  component: BlockedHint,
} satisfies Meta<typeof BlockedHint>;

export default meta;
type Story = StoryObj<typeof meta>;

export const OnKeyboardFocus: Story = {
  name: "Показывается при фокусе с клавиатуры",
  play: async ({ canvasElement }) => {
    await userEvent.tab();
    await expect(within(canvasElement).getByRole("button", { name: "Снять блок" })).toHaveFocus();
    const hint = await within(document.body).findByText("Нужна роль AppSec");
    await waitFor(() => expect(hint).toBeVisible());
  },
};

/** Остаётся открытой в момент axe-проверки. */
export const Open: Story = {
  args: { defaultOpen: true },
  play: async () => {
    const hint = await within(document.body).findByText("Нужна роль AppSec");
    await waitFor(() => expect(hint).toBeVisible());
  },
};
