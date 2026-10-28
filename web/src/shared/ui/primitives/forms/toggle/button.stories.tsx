import type { Meta, StoryObj } from "@storybook/react-vite";
import { SearchIcon } from "lucide-react";
import { expect, fn, userEvent, within } from "storybook/test";
import { Button } from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Button",
  component: Button,
  args: { children: "Опубликовать версию", onClick: fn() },
} satisfies Meta<typeof Button>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Primary: Story = {
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Опубликовать версию" }));
    await expect(args.onClick).toHaveBeenCalledTimes(1);
  },
};

export const Variants: Story = {
  render: () => (
    <div className="flex flex-wrap items-center gap-3">
      <Button variant="primary">Применить</Button>
      <Button variant="secondary">В избранное</Button>
      <Button variant="ghost">Открыть</Button>
      <Button variant="destructive">Удалить версию</Button>
    </div>
  ),
};

export const Sizes: Story = {
  render: () => (
    <div className="flex flex-wrap items-center gap-3">
      <Button size="default">Обычная 36 px</Button>
      <Button size="sm">Малая 32 px</Button>
      <Button size="icon" aria-label="Найти пакет">
        <SearchIcon />
      </Button>
      <Button size="icon-sm" aria-label="Найти пакет (малая)">
        <SearchIcon />
      </Button>
    </div>
  ),
};

export const Pending: Story = {
  name: "Pending: действие выполняется",
  args: { pending: true, children: "Повторить" },
  play: async ({ canvasElement }) => {
    const button = within(canvasElement).getByRole("button", { name: "Повторить" });
    await expect(button).toHaveAttribute("aria-busy", "true");
  },
};

export const Disabled: Story = {
  args: { disabled: true, children: "Недоступно" },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Недоступно" }));
    await expect(args.onClick).not.toHaveBeenCalled();
  },
};

export const KeyboardFocus: Story = {
  name: "Фокус с клавиатуры",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.tab();
    await expect(canvas.getByRole("button", { name: "Опубликовать версию" })).toHaveFocus();
  },
};
