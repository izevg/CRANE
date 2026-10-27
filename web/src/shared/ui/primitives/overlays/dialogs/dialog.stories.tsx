import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
import {
  Button,
  Dialog,
  DialogBody,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Dialog",
  component: Dialog,
  render: () => (
    <Dialog>
      <DialogTrigger render={<Button variant="secondary" />}>Создать токен</DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Новый токен</DialogTitle>
          <DialogDescription>Токен crp_a1b2c3d4 будет показан один раз</DialogDescription>
        </DialogHeader>
        <DialogBody>Срок действия: 90 дней. Область: чтение пакетов workspace platform.</DialogBody>
        <DialogFooter>
          <DialogClose render={<Button variant="secondary" />}>Отмена</DialogClose>
          <Button>Создать</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  ),
} satisfies Meta<typeof Dialog>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Создать токен" }));
    const dialog = await within(document.body).findByRole("dialog", { name: "Новый токен" });
    await waitFor(() => expect(dialog).toContainElement(document.activeElement as HTMLElement));
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull());
  },
};

export const EscapeCloses: Story = {
  name: "Esc закрывает",
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Создать токен" }));
    await within(document.body).findByRole("dialog");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull());
  },
};
