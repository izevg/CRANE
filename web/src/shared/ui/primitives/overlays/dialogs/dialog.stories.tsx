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

function TokenDialog({ defaultOpen = false }: { defaultOpen?: boolean }) {
  return (
    <Dialog defaultOpen={defaultOpen}>
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
  );
}

const meta = {
  title: "Примитивы/Dialog",
  component: TokenDialog,
} satisfies Meta<typeof TokenDialog>;

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

/** Остаётся открытым в момент axe-проверки: порталы и фон модалки попадают в контекст `body`. */
export const Open: Story = {
  args: { defaultOpen: true },
  play: async () => {
    const dialog = await within(document.body).findByRole("dialog", { name: "Новый токен" });
    await waitFor(() => expect(dialog).toContainElement(document.activeElement as HTMLElement));
  },
};
