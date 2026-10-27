import type { Meta, StoryObj } from "@storybook/react-vite";
import { useRef } from "react";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogBody,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
  Button,
} from "~/shared/ui/primitives";

function DeleteVersion({
  onConfirm,
  defaultOpen = false,
}: {
  onConfirm: () => void;
  defaultOpen?: boolean;
}) {
  const cancelRef = useRef<HTMLButtonElement>(null);
  return (
    <AlertDialog defaultOpen={defaultOpen}>
      <AlertDialogTrigger render={<Button variant="destructive" />}>
        Удалить версию
      </AlertDialogTrigger>
      <AlertDialogContent initialFocus={cancelRef}>
        <AlertDialogHeader>
          <AlertDialogTitle>Удалить версию 2.0.0-rc.1?</AlertDialogTitle>
          <AlertDialogDescription>
            Номер версии нельзя будет использовать повторно.
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogBody>
          <ul className="ml-5 list-disc">
            <li>Потребителей: 3</li>
            <li>Скачиваний за 30 дней: 214</li>
          </ul>
        </AlertDialogBody>
        <AlertDialogFooter>
          <AlertDialogCancel ref={cancelRef}>Отмена</AlertDialogCancel>
          <AlertDialogAction variant="destructive" onClick={onConfirm}>
            Удалить
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

const meta = {
  title: "Примитивы/AlertDialog",
  component: DeleteVersion,
  args: { onConfirm: fn() },
} satisfies Meta<typeof DeleteVersion>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DestructiveFocusOnCancel: Story = {
  name: "Деструктивный: фокус на «Отмена»",
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Удалить версию" }));
    const dialog = await within(document.body).findByRole("alertdialog", {
      name: "Удалить версию 2.0.0-rc.1?",
    });
    await waitFor(() =>
      expect(within(dialog).getByRole("button", { name: "Отмена" })).toHaveFocus(),
    );
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(within(document.body).queryByRole("alertdialog")).toBeNull());
  },
};

export const Confirm: Story = {
  play: async ({ canvasElement, args }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Удалить версию" }));
    const dialog = await within(document.body).findByRole("alertdialog");
    await userEvent.click(within(dialog).getByRole("button", { name: "Удалить" }));
    await expect(args.onConfirm).toHaveBeenCalledTimes(1);
  },
};

/** Остаётся открытым в момент axe-проверки. */
export const Open: Story = {
  args: { defaultOpen: true },
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog");
    await waitFor(() =>
      expect(within(dialog).getByRole("button", { name: "Отмена" })).toHaveFocus(),
    );
  },
};
