import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { SecretOnceDialog } from "~/shared/ui/dialogs";

const onDone = fn();

const meta = {
  title: "Диалоги/SecretOnceDialog",
  component: SecretOnceDialog,
  args: {
    secret: "crp_a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6",
    title: "Токен создан",
    onDone,
  },
} satisfies Meta<typeof SecretOnceDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Остаётся открытым в момент axe-проверки. */
export const Shown: Story = {
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog", { name: "Токен создан" });
    await expect(within(dialog).getByRole("button", { name: "Готово" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
  },
};

export const DoneAfterCheckbox: Story = {
  name: "«Готово» доступна после отметки",
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog");
    await userEvent.click(within(dialog).getByRole("checkbox", { name: "Я сохранил токен" }));
    await userEvent.click(within(dialog).getByRole("button", { name: "Готово" }));
    await expect(onDone).toHaveBeenCalled();
  },
};

export const Closed: Story = {
  name: "Без секрета диалога нет",
  args: { secret: null },
  play: async () => {
    await expect(within(document.body).queryByRole("alertdialog")).toBeNull();
  },
};

export const EscapeBlocked: Story = {
  name: "Esc до отметки не закрывает",
  play: async () => {
    await within(document.body).findByRole("alertdialog");
    await userEvent.keyboard("{Escape}");
    await expect(within(document.body).getByRole("alertdialog")).toBeVisible();
    await expect(onDone).not.toHaveBeenCalled();
  },
};

export const Dark: Story = { globals: { theme: "dark" } };
