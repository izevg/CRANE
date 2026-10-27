import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
import { Harness } from "./Harness";

const MIN_ERROR = /Минимум 20 символов/;
const MIN_ERROR_EN = /At least 20 characters/;
const LEFT = /Остал(ось|ся)/;

const meta = {
  title: "Фильтры/JustificationField",
  component: Harness,
  decorators: [
    (Story) => (
      <div className="w-96">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof Harness>;

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
  args: { initial: "Нужен доступ" },
  play: async ({ canvasElement }) => {
    const field = within(canvasElement).getByRole("textbox", { name: "Обоснование" });
    await userEvent.click(field);
    await userEvent.tab();
    await waitFor(() => expect(field).toHaveAccessibleDescription(MIN_ERROR));
  },
};

export const Enough: Story = {
  name: "Достаточно: остаток до предела",
  args: { initial: "Закрываем CVE-2026-31337 в релизе 1.4.2" },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("textbox", { name: "Обоснование" }),
    ).toHaveAccessibleDescription(LEFT);
  },
};

export const ErrorShown: Story = {
  name: "Ошибка показана сразу (попытка отправки)",
  args: { initial: "коротко", showError: true },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("textbox", { name: "Обоснование" }),
    ).toHaveAccessibleDescription(MIN_ERROR);
  },
};

export const English: Story = {
  name: "EN: подписи на английском",
  args: { initial: "short", showError: true },
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
  args: { initial: "коротко", showError: true },
};
