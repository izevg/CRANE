import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { DiffViewer } from "~/shared/ui/data-display";

const ADDED = /clsx/;
const REMOVED = /moment/;
const CHANGED = /изменено: было "\^4\.17\.20", стало "\^4\.17\.21"/;

const SIGN = /^[−+]$/;

const meta = {
  title: "Данные/DiffViewer",
  component: DiffViewer,
  args: {
    label: "Изменения манифеста 1.4.1 → 1.4.2",
    rows: [
      { kind: "same", path: "name", text: '"name": "@acme/ui"' },
      { kind: "change", path: "version", before: '"1.4.1"', after: '"1.4.2"' },
      { kind: "change", path: "dependencies.lodash", before: '"^4.17.20"', after: '"^4.17.21"' },
      { kind: "same", path: "dependencies.react", text: '"react": "^19.0.0"' },
      { kind: "add", path: "dependencies.clsx", text: '"clsx": "^2.1.1"' },
      { kind: "remove", path: "dependencies.moment", text: '"moment": "^2.30.1"' },
    ],
  },
} satisfies Meta<typeof DiffViewer>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Inline: Story = {
  name: "Подряд (inline)",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const added = canvas.getByRole("row", { name: ADDED });
    await expect(added).toHaveTextContent("+");
    await expect(added).toHaveTextContent("добавлено");
    await expect(canvas.getByRole("row", { name: REMOVED })).toHaveTextContent("удалено");
    const changed = canvas.getByRole("row", { name: CHANGED });
    await expect(changed).toBeVisible();
    // Знаки «−» и «+» стоят на одной высоте со своими строками значений
    const centerY = (element: Element) => {
      const box = element.getBoundingClientRect();
      return box.top + box.height / 2;
    };
    const signs = within(changed).getAllByText(SIGN);
    const before = within(changed).getByText('"^4.17.20"');
    const after = within(changed).getByText('"^4.17.21"');
    await expect(Math.abs(centerY(signs[0] as Element) - centerY(before))).toBeLessThanOrEqual(1);
    await expect(Math.abs(centerY(signs[1] as Element) - centerY(after))).toBeLessThanOrEqual(1);
  },
};

export const Split: Story = {
  name: "Рядом (split)",
  args: { mode: "split" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("columnheader", { name: "Стало" })).toBeVisible();
    // Изменённое поле — одна строка: имя поля не повторяется
    await expect(canvas.getAllByText("dependencies.lodash")).toHaveLength(1);
  },
};

export const Dark: Story = { globals: { theme: "dark" } };

export const SplitDark: Story = {
  name: "Рядом, тёмная тема",
  args: { mode: "split" },
  globals: { theme: "dark" },
};
