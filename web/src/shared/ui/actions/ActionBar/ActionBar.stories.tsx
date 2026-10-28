import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import { ActionBar } from "~/shared/ui/actions";

const STEP_UP_HINT = /требуется подтверждение входа/;

const meta = {
  title: "Действия/ActionBar",
  component: ActionBar,
  args: {
    actions: {
      primary: { label: "Скопировать команду установки", run: { onSelect: fn() } },
      secondary: [
        { label: "В избранное", run: { onSelect: fn() } },
        { label: "Открыть в GitLab", run: { href: "https://gitlab.example/platform/ui" } },
      ],
      menu: [
        { label: "Назначить замену…", run: { onSelect: fn() } },
        { label: "Запросить одобрение…", run: { onSelect: fn() } },
        { label: "Объявить устаревшим…", run: { onSelect: fn() }, destructive: true },
      ],
    },
  },
} satisfies Meta<typeof ActionBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Full: Story = {
  name: "Primary, два secondary и меню",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getAllByRole("button").map((b) => b.textContent)).toEqual([
      "Скопировать команду установки",
      "В избранное",
      "",
    ]);
    await expect(canvas.getByRole("link", { name: "Открыть в GitLab" })).toBeVisible();
  },
};

/** Остаётся открытой в момент axe-проверки; опасное действие — последним, за разделителем. */
export const MenuWithDestructive: Story = {
  name: "Меню: опасное действие последним",
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Другие действия" }));
    const menu = await within(document.body).findByRole("menu");
    const items = within(menu).getAllByRole("menuitem");
    await expect(items.at(-1)).toHaveTextContent("Объявить устаревшим…");
    await expect(within(menu).getByRole("separator")).toBeInTheDocument();
  },
};

export const StepUp: Story = {
  name: "Step-up: бейдж MFA",
  args: {
    actions: {
      primary: { label: "Применить retention", run: { onSelect: fn() }, stepUp: true },
    },
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("MFA")).toBeVisible();
    await expect(canvas.getByRole("button")).toHaveAccessibleName(STEP_UP_HINT);
  },
};

export const DisabledWithReason: Story = {
  name: "Недоступно с причиной",
  args: {
    actions: {
      primary: {
        label: "Выпустить токен",
        run: { onSelect: fn() },
        disabledReason: "Долгоживущие токены запрещены политикой",
      },
    },
  },
  play: async ({ canvasElement }) => {
    const button = within(canvasElement).getByRole("button", { name: "Выпустить токен" });
    await expect(button).toHaveAttribute("aria-disabled", "true");
    await expect(button).toHaveAccessibleDescription("Долгоживущие токены запрещены политикой");
    await userEvent.tab();
    await waitFor(() => expect(button).toHaveFocus());
  },
};

export const FlatRow: Story = {
  name: "Плоский ряд (массовые действия)",
  args: {
    actions: [
      { label: "Принять VEX", run: { onSelect: fn() } },
      { label: "Назначить исполнителя", run: { onSelect: fn() } },
      { label: "Исключение с TTL", run: { onSelect: fn() }, stepUp: true },
    ],
  },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getAllByRole("button")).toHaveLength(3);
  },
};

export const EmptyGroup: Story = {
  name: "Пустая группа — ничего не рисуется",
  args: { actions: {} },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole("button")).toBeNull();
    await expect(canvasElement.querySelector("[data-slot=action-bar]")).toBeNull();
  },
};
