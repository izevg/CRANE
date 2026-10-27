import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, userEvent, within } from "storybook/test";
import { SegmentedControl } from "~/shared/ui/inputs";

type Item = { value: string; label: string; count?: number | { count: number; capped: boolean } };

const VERSIONS: readonly Item[] = [
  { value: "all", label: "Все", count: 48 },
  { value: "released", label: "Выпущенные", count: 41 },
  { value: "quarantine", label: "Карантин", count: 2 },
  { value: "blocked", label: "Заблокированные", count: 1 },
  { value: "deleted", label: "Удалённые", count: 4 },
];

const PRIORITIES: readonly Item[] = [
  { value: "all", label: "Все" },
  { value: "p0", label: "P0", count: 1 },
  { value: "p1", label: "P1", count: 6 },
  { value: "p2", label: "P2", count: 23 },
  { value: "p3", label: "P3", count: 51 },
  { value: "p4", label: "P4", count: 140 },
];

const CAPPED: readonly Item[] = [
  { value: "all", label: "Все", count: { count: 1000, capped: true } },
  { value: "mine", label: "Мои", count: 12 },
];

type Props = { label: string; items: readonly Item[]; initial: string };

function Controlled({ label, items, initial }: Props) {
  const [value, setValue] = useState(initial);
  return <SegmentedControl label={label} items={items} value={value} onValueChange={setValue} />;
}

const meta = {
  title: "Фильтры/SegmentedControl",
  component: Controlled,
} satisfies Meta<typeof Controlled>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Versions: Story = {
  name: "Версии пакета со счётчиками",
  args: { label: "Состояние версии", items: VERSIONS, initial: "all" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Выпущенные 41" }));
    await expect(canvas.getByRole("button", { name: "Выпущенные 41" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await expect(canvas.getByRole("button", { name: "Все 48" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  },
};

export const Priorities: Story = {
  name: "Приоритеты находок",
  args: { label: "Приоритет", items: PRIORITIES, initial: "all" },
};

export const Capped: Story = {
  name: "Усечённый счётчик «1 000+»",
  args: { label: "Владелец", items: CAPPED, initial: "all" },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("button", { name: "Все 1 000+" })).toBeVisible();
  },
};

export const Keyboard: Story = {
  name: "Клавиатура: стрелки и Space меняют выбор",
  args: { label: "Приоритет", items: PRIORITIES, initial: "all" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.tab();
    await userEvent.keyboard("{ArrowRight}");
    await userEvent.keyboard(" ");
    await expect(canvas.getByRole("button", { name: "P0 1" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await userEvent.keyboard("{ArrowRight}");
    await userEvent.keyboard("{Enter}");
    await expect(canvas.getByRole("button", { name: "P1 6" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  },
};

export const NoDeselect: Story = {
  name: "Повторное нажатие не снимает выбор",
  args: { label: "Состояние версии", items: VERSIONS, initial: "released" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("button", { name: "Выпущенные 41" }));
    await expect(canvas.getByRole("button", { name: "Выпущенные 41" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: { label: "Состояние версии", items: VERSIONS, initial: "released" },
};
