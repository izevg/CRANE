import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { StatusBadge } from "~/shared/ui/status";

const meta = {
  title: "Статус/StatusBadge",
  component: StatusBadge,
  args: { tone: "success", label: "Выпущена" },
} satisfies Meta<typeof StatusBadge>;

export default meta;
type Story = StoryObj<typeof meta>;

const Tones = () => (
  <div className="flex flex-wrap items-center gap-3">
    <StatusBadge tone="success" label="Выпущена" />
    <StatusBadge tone="info" label="Идёт проверка" />
    <StatusBadge tone="warning" label="Карантин" />
    <StatusBadge tone="error" label="Заблокирована" />
    <StatusBadge tone="neutral" label="Удалена" />
  </div>
);

export const AllTones: Story = {
  name: "Все тоны",
  render: () => <Tones />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(
      canvas
        .getByText("Карантин")
        .closest("[data-tone]")
        ?.querySelector('[data-slot="status-icon"]'),
    ).not.toBeNull();
    await expect(
      canvas
        .getByText("Заблокирована")
        .closest("[data-tone]")
        ?.querySelector('[data-slot="status-icon"]'),
    ).not.toBeNull();
    await expect(
      canvas
        .getByText("Выпущена")
        .closest("[data-tone]")
        ?.querySelector('[data-slot="status-dot"]'),
    ).not.toBeNull();
  },
};

export const AllTonesDark: Story = {
  name: "Все тоны, тёмная тема",
  render: () => <Tones />,
  globals: { theme: "dark" },
};

export const Pulse: Story = {
  name: "Пульс у выполняющегося процесса",
  args: { tone: "info", label: "Перенос хранилища", pulse: true },
  play: async ({ canvasElement }) => {
    await expect(canvasElement.querySelector('[data-pulse="true"]')).not.toBeNull();
  },
};
