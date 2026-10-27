import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { CommandPaletteView } from "~/widgets/command-palette";

const meta = {
  title: "Виджеты/CommandPaletteView",
  component: CommandPaletteView,
  args: {
    open: true,
    onOpenChange: fn(),
    onQueryChange: fn(),
    onNavigate: fn(),
    navigation: [
      { id: "search", label: "Поиск", href: "/search" },
      { id: "tokens", label: "Мои токены", href: "/tokens" },
      { id: "packages", label: "Мои пакеты", href: "/packages" },
    ],
    packages: [
      {
        id: "acme-ui",
        name: "@acme/ui",
        format: "npm",
        latest: "1.4.2",
        href: "/packages/acme-ui",
      },
      { id: "lodash", name: "lodash", format: "npm", latest: "4.17.21", href: "/packages/lodash" },
    ],
    actions: [{ label: "Создать токен", run: { onSelect: fn() } }],
  },
} satisfies Meta<typeof CommandPaletteView>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Остаётся открытой в момент axe-проверки. */
export const Open: Story = {
  play: async () => {
    await expect(await within(document.body).findByRole("combobox")).toBeInTheDocument();
  },
};

export const Typing: Story = {
  name: "Ввод фильтрует разделы",
  play: async () => {
    const input = await within(document.body).findByRole("combobox");
    await userEvent.type(input, "ток");
    await expect(within(document.body).getByText("Мои токены")).toBeInTheDocument();
  },
};

export const Loading: Story = {
  args: { packages: [], packagesLoading: true },
};
