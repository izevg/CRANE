import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { DiffViewer } from "~/shared/ui/data-display";

const ADDED = /\^4\.17\.21/;
const REMOVED = /\^4\.17\.20/;

const meta = {
  title: "Данные/DiffViewer",
  component: DiffViewer,
  args: {
    label: "Изменения манифеста 1.4.1 → 1.4.2",
    rows: [
      { kind: "same", path: "name", text: '"name": "@acme/ui"' },
      { kind: "remove", path: "version", text: '"version": "1.4.1"' },
      { kind: "add", path: "version", text: '"version": "1.4.2"' },
      { kind: "remove", path: "dependencies.lodash", text: '"lodash": "^4.17.20"' },
      { kind: "add", path: "dependencies.lodash", text: '"lodash": "^4.17.21"' },
      { kind: "same", path: "dependencies.react", text: '"react": "^19.0.0"' },
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
  },
};

export const Split: Story = {
  name: "Рядом (split)",
  args: { mode: "split" },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("columnheader", { name: "Стало" })).toBeVisible();
  },
};

export const Dark: Story = { globals: { theme: "dark" } };

export const SplitDark: Story = {
  name: "Рядом, тёмная тема",
  args: { mode: "split" },
  globals: { theme: "dark" },
};
