import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import { CommandPaletteView } from "~/widgets/command-palette";

const STEP_UP_OPTION = /Отозвать все токены/;
const STEP_UP_HINT = /требуется подтверждение входа/;
const BLOCKED_OPTION = /Снять блокировку/;
const SEARCH_OPTION = /Поиск/;

const onSelectToken = fn();
const onSelectBlocked = fn();

const meta = {
  title: "Шелл/Палитра",
  component: CommandPaletteView,
  parameters: { layout: "fullscreen" },
  args: {
    open: true,
    onOpenChange: fn(),
    onQueryChange: fn(),
    onNavigate: fn(),
    navigation: [
      { id: "search", label: "Поиск", href: "/search" },
      { id: "tokens", label: "Мои токены", href: "/tokens", hint: "Доступ" },
      { id: "packages", label: "Мои пакеты", href: "/packages" },
      { id: "findings", label: "Находки", href: "/findings", hint: "Безопасность" },
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
    actions: [
      { label: "Создать токен", run: { onSelect: onSelectToken } },
      { label: "Отозвать все токены", run: { onSelect: fn() }, stepUp: true, destructive: true },
      {
        label: "Снять блокировку пакета",
        run: { onSelect: onSelectBlocked },
        disabledReason: "Нужна роль AppSec",
      },
    ],
  },
} satisfies Meta<typeof CommandPaletteView>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Остаётся открытой в момент axe-проверки: все три группы, step-up и недоступное действие. */
export const Open: Story = {
  play: async ({ args }) => {
    const body = within(document.body);
    const input = await body.findByRole("combobox");
    await expect(input).toHaveAccessibleName("Перейти, найти пакет или действие");
    await expect(body.getByRole("option", { name: STEP_UP_OPTION })).toHaveAccessibleName(
      STEP_UP_HINT,
    );
    await expect(body.getByRole("option", { name: BLOCKED_OPTION })).toHaveAttribute(
      "aria-disabled",
      "true",
    );

    await userEvent.click(input);
    await userEvent.keyboard("{ArrowDown}");
    const active = body.getByRole("option", { name: SEARCH_OPTION });
    await waitFor(() => expect(active).toHaveAttribute("data-highlighted"));
    // Проверка CSS, которой нет в jsdom: контур активного пункта нарисован.
    await expect(getComputedStyle(active).outlineStyle).toBe("solid");
    await userEvent.keyboard("{Enter}");
    await expect(args.onNavigate).toHaveBeenCalledWith("/search");
  },
};

export const Typing: Story = {
  name: "Ввод фильтрует разделы",
  play: async ({ args }) => {
    const input = await within(document.body).findByRole("combobox");
    await userEvent.type(input, "ток");
    await expect(args.onQueryChange).toHaveBeenLastCalledWith("ток");
    await expect(within(document.body).getByText("Мои токены")).toBeInTheDocument();
    await expect(within(document.body).queryByText("Мои пакеты")).not.toBeInTheDocument();
  },
};

export const PackageSearching: Story = {
  name: "Пакеты загружаются",
  args: { packages: [], packagesLoading: true },
};

export const NothingFound: Story = {
  name: "Ничего не найдено",
  args: { navigation: [], packages: [], actions: [] },
  play: async () => {
    const input = await within(document.body).findByRole("combobox");
    await userEvent.type(input, "яяяя");
    await expect(await within(document.body).findByText("Ничего не найдено")).toBeInTheDocument();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
};

const INSTANT_MAX_SECONDS = 0.1;

function seconds(value: string): number {
  return Math.max(...value.split(",").map((part) => parseFloat(part)));
}

/** Палитру открывают по Ctrl+K десятки раз: без анимации входа/выхода (spec 2), в отличие от диалога. */
export const NoMotion: Story = {
  name: "Движение: палитра без анимации",
  play: async () => {
    await within(document.body).findByRole("combobox");
    const popup = document.querySelector("[data-slot=dialog-content]");
    if (!(popup instanceof HTMLElement)) {
      throw new Error("нет окна палитры");
    }
    await expect(seconds(getComputedStyle(popup).transitionDuration)).toBeLessThanOrEqual(
      INSTANT_MAX_SECONDS,
    );
    await expect(getComputedStyle(popup).transform).not.toContain("0.9");
  },
};
