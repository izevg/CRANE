import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { StepUpDialog } from "../index";

const onContinue = fn();

const meta = {
  title: "Диалоги/StepUpDialog",
  component: StepUpDialog,
  args: {
    open: true,
    onOpenChange: fn(),
    actionLabel: "Раскрыть данные события",
    onContinue,
  },
} satisfies Meta<typeof StepUpDialog>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Остаётся открытым в момент axe-проверки. */
export const Open: Story = {
  name: "Открыт",
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog", {
      name: "Подтвердите вход",
    });
    await expect(
      within(dialog).getByRole("button", { name: "Продолжить в Keycloak" }),
    ).toBeVisible();
    await expect(within(dialog).queryByRole("textbox")).toBeNull();
  },
};

export const Continue: Story = {
  name: "Продолжить ведёт в Keycloak",
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog");
    await userEvent.click(within(dialog).getByRole("button", { name: "Продолжить в Keycloak" }));
    await expect(onContinue).toHaveBeenCalled();
  },
};

export const English: Story = {
  name: "EN: подписи на английском",
  globals: { locale: "en" },
  play: async () => {
    const dialog = await within(document.body).findByRole("alertdialog", {
      name: "Confirm sign-in",
    });
    await expect(
      within(dialog).getByRole("button", { name: "Continue in Keycloak" }),
    ).toBeVisible();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
};
