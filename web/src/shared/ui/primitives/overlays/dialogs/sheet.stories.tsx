import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
import {
  Button,
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Sheet",
  component: Sheet,
  render: () => (
    <Sheet>
      <SheetTrigger render={<Button variant="secondary" />}>Открыть событие</SheetTrigger>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>Событие 6003</SheetTitle>
          <SheetDescription>request_id 0192f3a4-7b1c-7e21</SheetDescription>
        </SheetHeader>
        <SheetBody>
          Токен crr_9f8e7d6c отозван. Исполнитель: Петрова А., 27 окт, 14:05 UTC.
        </SheetBody>
        <SheetFooter>
          <Button variant="secondary">Копировать request_id</Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  ),
} satisfies Meta<typeof Sheet>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Right: Story = {
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Открыть событие" }));
    const sheet = await within(document.body).findByRole("dialog", { name: "Событие 6003" });
    await expect(sheet).toHaveAccessibleDescription("request_id 0192f3a4-7b1c-7e21");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull());
  },
};

export const EscapeCloses: Story = {
  name: "Esc закрывает",
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Открыть событие" }));
    await within(document.body).findByRole("dialog");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull());
  },
};
