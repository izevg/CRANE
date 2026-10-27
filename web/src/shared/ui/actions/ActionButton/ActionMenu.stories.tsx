import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import { ActionMenu } from "~/shared/ui/actions";

const meta = {
  title: "Действия/ActionMenu",
  component: ActionMenu,
  args: {
    actions: [
      { label: "Объявить устаревшим…", run: { onSelect: fn() }, destructive: true },
      { label: "Назначить замену…", run: { onSelect: fn() } },
      { label: "Открыть в GitLab", run: { href: "https://gitlab.example/platform/ui" } },
      { label: "Применить retention", run: { onSelect: fn() }, stepUp: true },
      {
        label: "Снять карантин",
        run: { onSelect: fn() },
        disabledReason: "Нужна роль AppSec",
      },
    ],
  },
} satisfies Meta<typeof ActionMenu>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DestructiveLast: Story = {
  name: "Опасное — последним, за разделителем",
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Другие действия" }));
    const menu = await within(document.body).findByRole("menu");
    const items = within(menu).getAllByRole("menuitem");
    await expect(items.at(-1)).toHaveTextContent("Объявить устаревшим…");
    await expect(within(menu).getByRole("separator")).toBeInTheDocument();
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(within(document.body).queryByRole("menu")).toBeNull());
  },
};

/** Остаётся открытым в момент axe-проверки. */
export const Open: Story = {
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Другие действия" }));
    await within(document.body).findByRole("menu");
  },
};

export const Empty: Story = {
  name: "Пустой список — ничего не рисуется",
  args: { actions: [] },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).queryByRole("button", { name: "Другие действия" }),
    ).toBeNull();
  },
};
