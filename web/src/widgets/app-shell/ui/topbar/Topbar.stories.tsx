import type { Meta, StoryObj } from "@storybook/react-vite";
import { BellIcon } from "lucide-react";
import { expect, fn, userEvent, within } from "storybook/test";
import { Button } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { Topbar } from "./Topbar";

const onOpenPalette = fn();
const SEARCH = /Поиск/;

const meta = {
  title: "Шелл/Topbar",
  component: Topbar,
  parameters: { layout: "fullscreen" },
  args: {
    breadcrumbs: {
      trail: [
        { label: "Пакеты", href: "/packages" },
        { label: "@acme/ui", href: "/packages/acme-ui" },
      ],
      current: "Версии",
    },
    onOpenPalette,
    sidebarToggle: (
      <Button variant="ghost" size="icon" aria-label="Главное меню" aria-expanded>
        ☰
      </Button>
    ),
  },
} satisfies Meta<typeof Topbar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  name: "Крошки и поиск",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Версии")).toHaveAttribute("aria-current", "page");
    await expect(canvas.queryByRole("link", { name: "Версии" })).toBeNull();
    await expect(canvas.getByRole("link", { name: "Пакеты" })).toHaveAttribute("href", "/packages");
    await userEvent.click(canvas.getByRole("button", { name: "Поиск и переходы Ctrl K" }));
    await expect(onOpenPalette).toHaveBeenCalled();
  },
};

export const WithoutPalette: Story = {
  name: "Без палитры: кнопки поиска нет",
  render: ({ onOpenPalette: _palette, ...rest }) => <Topbar {...rest} />,
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole("button", { name: SEARCH })).toBeNull();
  },
};

export const WithSlots: Story = {
  name: "Все слоты: workspace, статус, уведомления, аккаунт",
  args: {
    workspace: (
      <Button variant="secondary" size="sm">
        Workspace: platform
      </Button>
    ),
    status: <StatusBadge tone="success" label="Режим: норма" />,
    notifications: (
      <Button variant="ghost" size="icon" aria-label="Уведомления: 3">
        <BellIcon aria-hidden="true" />
      </Button>
    ),
    account: (
      <Button variant="ghost" size="icon" aria-label="АС, аккаунт Анны Смирновой">
        АС
      </Button>
    ),
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("button", { name: "Workspace: platform" })).toBeVisible();
    await expect(canvas.getByText("Режим: норма")).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Уведомления: 3" })).toBeVisible();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
};
