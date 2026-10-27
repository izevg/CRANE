import type { Meta, StoryObj } from "@storybook/react-vite";
import { type ComponentProps, useState } from "react";
import { expect, userEvent, waitFor, within } from "storybook/test";
import { JustificationField } from "./JustificationField";

const MIN_ERROR = /Минимум 20 символов/;
const MIN_ERROR_EN = /At least 20 characters/;
const MAX_ERROR = /Максимум 2\s000 символов/;
const LEFT = /Остал(ось|ся)/;

/** Форма держит значение сама; в истории это локальное состояние, стартующее с `args.value`. */
function Controlled(props: ComponentProps<typeof JustificationField>) {
  const [value, setValue] = useState(props.value);
  return <JustificationField {...props} value={value} onChange={setValue} />;
}

const meta = {
  title: "Фильтры/JustificationField",
  component: JustificationField,
  render: (args) => <Controlled {...args} />,
  args: { label: "Обоснование", value: "", onChange: () => undefined },
  decorators: [
    (Story) => (
      <div className="w-96">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof JustificationField>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Empty: Story = {
  name: "Пусто",
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("textbox", { name: "Обоснование" }),
    ).toHaveAccessibleDescription("0 из 20");
  },
};

export const TooShort: Story = {
  name: "Недобор: ошибка после ухода из поля",
  args: { value: "Нужен доступ" },
  play: async ({ canvasElement }) => {
    const field = within(canvasElement).getByRole("textbox", { name: "Обоснование" });
    await userEvent.click(field);
    await userEvent.tab();
    await waitFor(() => expect(field).toHaveAccessibleDescription(MIN_ERROR));
  },
};

export const Enough: Story = {
  name: "Достаточно: остаток до предела",
  args: { value: "Закрываем CVE-2026-31337 в релизе 1.4.2" },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("textbox", { name: "Обоснование" }),
    ).toHaveAccessibleDescription(LEFT);
  },
};

export const ErrorShown: Story = {
  name: "Ошибка показана сразу (попытка отправки)",
  args: { value: "коротко", showError: true },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("textbox", { name: "Обоснование" }),
    ).toHaveAccessibleDescription(MIN_ERROR);
  },
};

export const TooLong: Story = {
  name: "Превышение: ошибка «Максимум 2000 символов», ввод не обрезается",
  args: { value: "а".repeat(2001) },
  play: async ({ canvasElement }) => {
    const field = within(canvasElement).getByRole("textbox", { name: "Обоснование" });
    await expect(field).not.toHaveAttribute("maxlength");
    await expect(field).toHaveAccessibleDescription(MAX_ERROR);
  },
};

export const English: Story = {
  name: "EN: подписи на английском",
  args: { value: "short", showError: true },
  globals: { locale: "en" },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("textbox", { name: "Обоснование" }),
    ).toHaveAccessibleDescription(MIN_ERROR_EN);
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: { value: "коротко", showError: true },
};
