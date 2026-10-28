import type { Meta, StoryObj } from "@storybook/react-vite";
import {
  ActivityIcon,
  KeyRoundIcon,
  PackageIcon,
  ScrollTextIcon,
  SearchIcon,
  SettingsIcon,
  ShieldAlertIcon,
} from "lucide-react";
import { expect, userEvent, waitFor, within } from "storybook/test";
import { Sidebar } from "./Sidebar";

const THEME_BUTTON = /^Тема:/;
const CRANE_LINK = /CRANE/;

const meta = {
  title: "Шелл/Sidebar",
  component: Sidebar,
  parameters: { layout: "fullscreen" },
  decorators: [
    (Story) => (
      <div className="h-svh w-62">
        <Story />
      </div>
    ),
  ],
  args: {
    collapsed: false,
    helpHref: "/help",
    account: {
      name: "Анна Смирнова-Остроградская",
      role: "владелец библиотеки @acme/ui и сопровождающий",
    },
    groups: [
      {
        id: "registry",
        label: "Реестр",
        items: [
          { id: "search", label: "Поиск", href: "/search", icon: SearchIcon },
          { id: "packages", label: "Пакеты", href: "/packages", icon: PackageIcon, active: true },
        ],
      },
      {
        id: "security",
        label: "Безопасность",
        items: [
          { id: "findings", label: "Находки", href: "/findings", icon: ShieldAlertIcon },
          { id: "audit", label: "Журнал", href: "/audit", icon: ScrollTextIcon },
          { id: "platform", label: "Платформа", href: "/platform", icon: ActivityIcon },
        ],
      },
    ],
    footerItems: [
      { id: "tokens", label: "Мои токены", href: "/me/tokens", icon: KeyRoundIcon },
      { id: "settings", label: "Настройки", href: "/settings", icon: SettingsIcon },
    ],
  },
} satisfies Meta<typeof Sidebar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Expanded: Story = {
  name: "Развёрнут: активный пункт и «Справка»",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("link", { name: "Пакеты" })).toHaveAttribute(
      "aria-current",
      "page",
    );
    await expect(canvas.getByRole("link", { name: "Справка" })).toHaveAttribute("href", "/help");
    await expect(canvas.getAllByRole("link", { name: CRANE_LINK })).toHaveLength(1);
  },
};

export const Rail: Story = {
  name: "Рельса 64 px: подписи остаются в доступных именах",
  args: { collapsed: true },
  decorators: [
    (Story) => (
      <div className="h-svh w-16">
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("link", { name: "Пакеты" })).toBeVisible();
    await expect(canvas.getByRole("link", { name: "Справка" })).toBeVisible();
  },
};

function rightEdge(element: Element): number {
  return element.getBoundingClientRect().right;
}

export const LongUserBlock: Story = {
  name: "Блок пользователя: длинные имя и роль не выходят за 248 px",
  play: async ({ canvasElement }) => {
    const block = canvasElement.querySelector("[data-slot=sidebar-user]");
    const sidebar = canvasElement.querySelector("[data-slot=sidebar]");
    await expect(block).not.toBeNull();
    await expect(sidebar).not.toBeNull();
    if (block === null || sidebar === null) {
      return;
    }
    await expect(rightEdge(block)).toBeLessThanOrEqual(rightEdge(sidebar) + 0.5);
    const name = within(block as HTMLElement).getByText("Анна Смирнова-Остроградская");
    // обрезка многоточием: текст шире видимой области, полный текст — в title
    await expect(name.scrollWidth).toBeGreaterThan(name.clientWidth);
    await expect(name).toHaveAttribute("title", "Анна Смирнова-Остроградская");
    await expect(getComputedStyle(name).textOverflow).toBe("ellipsis");
  },
};

export const UserBlockRail: Story = {
  name: "Блок пользователя в рельсе 64 px: только аватар",
  args: { collapsed: true },
  decorators: [
    (Story) => (
      <div className="h-svh w-16">
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    const block = canvasElement.querySelector("[data-slot=sidebar-user]");
    const sidebar = canvasElement.querySelector("[data-slot=sidebar]");
    if (block === null || sidebar === null) {
      throw new Error("нет блока пользователя или сайдбара");
    }
    await expect(rightEdge(block)).toBeLessThanOrEqual(rightEdge(sidebar) + 0.5);
    await expect(
      within(canvasElement).getByRole("img", {
        name: "Анна Смирнова-Остроградская, владелец библиотеки @acme/ui и сопровождающий",
      }),
    ).toBeVisible();
  },
};

/** Остаётся открытой в момент axe-проверки. */
export const ThemeMenuOpen: Story = {
  name: "Меню темы открыто",
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: THEME_BUTTON }));
    const item = await within(document.body).findByRole("menuitemradio", { name: "Тёмная" });
    await userEvent.click(item);
    await waitFor(() => expect(document.documentElement).toHaveAttribute("data-theme", "dark"));
    await expect(
      within(document.body).getByRole("menuitemradio", { name: "Тёмная" }),
    ).toHaveAttribute("aria-checked", "true");
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
};
