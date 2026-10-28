import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import { Field, FieldDescription, FieldLabel, PasswordInput } from "~/shared/ui/primitives";

const TOKEN = "crb_9f2c71aa41d0e58b";

const meta = {
  title: "Примитивы/PasswordInput",
  component: PasswordInput,
  render: (args) => (
    <Field name="token" className="w-96 max-w-full">
      <FieldLabel>Токен доступа</FieldLabel>
      <PasswordInput autoComplete="current-password" {...args} />
      <FieldDescription>Bootstrap-токен из журнала сервера или CRANE_TOKEN</FieldDescription>
    </Field>
  ),
} satisfies Meta<typeof PasswordInput>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Hidden: Story = {
  name: "Значение скрыто",
  args: { defaultValue: TOKEN },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByLabelText("Токен доступа")).toHaveAttribute("type", "password");
    await expect(canvas.getByRole("button", { name: "Показать пароль" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  },
};

export const Shown: Story = {
  name: "Значение показано",
  args: { defaultValue: TOKEN },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Показать пароль" }));
    await expect(canvas.getByLabelText("Токен доступа")).toHaveAttribute("type", "text");
    await expect(canvas.getByRole("button", { name: "Показать пароль" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  },
};

export const TokenToggle: Story = {
  name: "Своё имя кнопки: «Показать токен»",
  args: { toggleLabel: "Показать токен" },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("button", { name: "Показать токен" }),
    ).toBeVisible();
  },
};

export const Paste: Story = {
  name: "Вставка не режется",
  play: async ({ canvasElement }) => {
    const input = within(canvasElement).getByLabelText("Токен доступа");
    await userEvent.click(input);
    await userEvent.paste(`${TOKEN}${TOKEN}${TOKEN}${TOKEN}`);
    await expect(input).toHaveValue(`${TOKEN}${TOKEN}${TOKEN}${TOKEN}`);
  },
};

export const English: Story = {
  name: "EN",
  globals: { locale: "en" },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("button", { name: "Show password" }),
    ).toBeVisible();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: { defaultValue: TOKEN },
};
