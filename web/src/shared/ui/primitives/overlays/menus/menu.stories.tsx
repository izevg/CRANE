import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import {
  Button,
  Menu,
  MenuContent,
  MenuGroup,
  MenuItem,
  MenuLabel,
  MenuSeparator,
  MenuTrigger,
} from "~/shared/ui/primitives";

function ActionsMenu({ onSelect }: { onSelect: () => void }) {
  return (
    <Menu>
      <MenuTrigger render={<Button variant="secondary" />}>Действия</MenuTrigger>
      <MenuContent>
        <MenuGroup>
          <MenuLabel>Пакет @acme/ui</MenuLabel>
          <MenuItem onClick={onSelect}>Назначить замену…</MenuItem>
          <MenuItem>Запросить одобрение…</MenuItem>
        </MenuGroup>
        <MenuSeparator />
        <MenuItem variant="destructive">Объявить устаревшим…</MenuItem>
      </MenuContent>
    </Menu>
  );
}

const meta = {
  title: "Примитивы/Menu",
  component: ActionsMenu,
  args: { onSelect: fn() },
} satisfies Meta<typeof ActionsMenu>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement, args }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Действия" }));
    await userEvent.click(
      await within(document.body).findByRole("menuitem", { name: "Назначить замену…" }),
    );
    await expect(args.onSelect).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(within(document.body).queryByRole("menu")).toBeNull());
  },
};

export const KeyboardNavigation: Story = {
  name: "Стрелки и Esc",
  play: async ({ canvasElement }) => {
    const trigger = within(canvasElement).getByRole("button", { name: "Действия" });
    trigger.focus();
    await userEvent.keyboard("{ArrowDown}");
    await within(document.body).findByRole("menu");
    await userEvent.keyboard("{ArrowDown}");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(within(document.body).queryByRole("menu")).toBeNull());
  },
};
