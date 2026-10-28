import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { desktop, screenParameters } from "../frame/screen";
import { Screen } from "./S5Page";

const meta = {
  title: "Экраны/S5 Мои токены",
  component: Screen,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof Screen>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Диалог остаётся открытым в момент axe-проверки. */
export const SecretShown: Story = {
  name: "Секрет показан один раз",
  args: { secret: true },
  play: async () => {
    await expect(
      await within(document.body).findByRole("alertdialog", { name: "Токен создан" }),
    ).toBeVisible();
  },
};

/** Sheet остаётся открытым в момент axe-проверки. */
export const NewTokenSheetOpen: Story = {
  name: "Sheet «Новый токен»",
  args: { sheet: true },
  play: async () => {
    await expect(
      await within(document.body).findByRole("dialog", { name: "Новый токен" }),
    ).toBeVisible();
  },
};
