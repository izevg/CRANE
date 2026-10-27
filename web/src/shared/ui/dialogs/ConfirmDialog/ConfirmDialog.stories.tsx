import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import { ConfirmDialog } from "~/shared/ui/dialogs";
import { Field, FieldLabel, Textarea } from "~/shared/ui/primitives";

const onConfirm = fn();

const meta = {
  title: "Диалоги/ConfirmDialog",
  component: ConfirmDialog,
  args: {
    open: true,
    onOpenChange: fn(),
    title: "Заблокировать версию lodash 4.17.20?",
    description: "Версия станет недоступна для установки.",
    consequences: [
      "Потребителей: 63",
      "Скачиваний за 30 дней: 12 340",
      "Основание: CVE-2026-31337",
    ],
    confirm: { label: "Заблокировать", run: { onSelect: onConfirm }, destructive: true },
  },
} satisfies Meta<typeof ConfirmDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Остаётся открытым в момент axe-проверки. Деструктивное действие: фокус на «Отмена». */
export const Destructive: Story = {
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog");
    await waitFor(() =>
      expect(within(dialog).getByRole("button", { name: "Отмена" })).toHaveFocus(),
    );
  },
};

export const Regular: Story = {
  args: {
    title: "Применить retention?",
    description: "Операция требует повторного подтверждения входа.",
    consequences: ["Будет удалено 214 старых снапшотов"],
    confirm: { label: "Применить", run: { onSelect: onConfirm }, stepUp: true },
  },
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog");
    await expect(within(dialog).getByText("MFA")).toBeVisible();
  },
};

export const Pending: Story = {
  args: { pending: true },
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog");
    await expect(within(dialog).getByRole("button", { name: "Заблокировать" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
  },
};

export const ConfirmCalls: Story = {
  name: "Подтверждение вызывает действие",
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog");
    await userEvent.click(within(dialog).getByRole("button", { name: "Заблокировать" }));
    await expect(onConfirm).toHaveBeenCalled();
  },
};

export const WithReason: Story = {
  name: "С полем причины: фокус на поле",
  args: {
    title: "Продлить исключение?",
    consequences: ["Срок исключения увеличится на 30 дней"],
    confirm: { label: "Продлить", run: { onSelect: onConfirm } },
    children: (
      <Field>
        <FieldLabel>Причина</FieldLabel>
        <Textarea />
      </Field>
    ),
  },
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog");
    await waitFor(() =>
      expect(within(dialog).getByRole("textbox", { name: "Причина" })).toHaveFocus(),
    );
  },
};

export const PendingCancel: Story = {
  name: "Выполняется: «Отмена» объясняет причину",
  args: { pending: true },
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog");
    await expect(
      within(dialog).getByRole("button", { name: "Отмена" }),
    ).toHaveAccessibleDescription("Выполняется…");
  },
};

export const Dark: Story = { globals: { theme: "dark" } };
