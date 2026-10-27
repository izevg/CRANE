import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
import {
  Button,
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverTitle,
  PopoverTrigger,
} from "~/shared/ui/primitives";

function StatusPopover({ defaultOpen = false }: { defaultOpen?: boolean }) {
  return (
    <Popover defaultOpen={defaultOpen}>
      <PopoverTrigger render={<Button variant="secondary" />}>Состояние</PopoverTrigger>
      <PopoverContent>
        <PopoverTitle>Режим: норма</PopoverTitle>
        <PopoverDescription>Все компоненты работают, последняя проверка 14:05.</PopoverDescription>
      </PopoverContent>
    </Popover>
  );
}

const meta = {
  title: "Примитивы/Popover",
  component: StatusPopover,
} satisfies Meta<typeof StatusPopover>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Состояние" }));
    const text = await within(document.body).findByText("Режим: норма");
    await waitFor(() => expect(text).toBeVisible());
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(within(document.body).queryByText("Режим: норма")).toBeNull());
  },
};

/** Остаётся открытым в момент axe-проверки. */
export const Open: Story = {
  args: { defaultOpen: true },
  play: async () => {
    await waitFor(async () =>
      expect(await within(document.body).findByText("Режим: норма")).toBeVisible(),
    );
  },
};
