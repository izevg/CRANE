import type { Meta, StoryObj } from "@storybook/react-vite";
import { useCallback, useState } from "react";
import { expect, userEvent, within } from "storybook/test";
import { FacetList } from "~/shared/ui/inputs";

type Item = { value: string; label: string; count: { count: number; capped: boolean } };

const FORMATS: readonly Item[] = [
  { value: "npm", label: "npm", count: { count: 1000, capped: true } },
  { value: "pypi", label: "PyPI", count: { count: 412, capped: false } },
  { value: "maven", label: "Maven", count: { count: 238, capped: false } },
  { value: "nuget", label: "NuGet", count: { count: 64, capped: false } },
  { value: "cargo", label: "Cargo", count: { count: 17, capped: false } },
];

const WORKSPACES: readonly Item[] = [
  { value: "platform", label: "platform", count: { count: 902, capped: false } },
  { value: "billing", label: "billing", count: { count: 318, capped: false } },
  { value: "mobile", label: "mobile", count: { count: 75, capped: false } },
];

type Props = { title: string; items: readonly Item[]; initial: readonly string[] };

function Controlled({ title, items, initial }: Props) {
  const [selected, setSelected] = useState<readonly string[]>(initial);
  const toggle = useCallback(
    (value: string) =>
      setSelected((prev) =>
        prev.includes(value) ? prev.filter((item) => item !== value) : [...prev, value],
      ),
    [],
  );
  return (
    <div className="w-72">
      <FacetList title={title} items={items} selected={selected} onToggle={toggle} />
    </div>
  );
}

const meta = {
  title: "Фильтры/FacetList",
  component: Controlled,
} satisfies Meta<typeof Controlled>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Formats: Story = {
  name: "По форматам",
  args: { title: "По форматам", items: FORMATS, initial: [] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const pypi = canvas.getByRole("button", { name: "PyPI 412" });
    await expect(pypi).toHaveAttribute("aria-pressed", "false");
    await userEvent.click(pypi);
    await expect(pypi).toHaveAttribute("aria-pressed", "true");
    await userEvent.click(pypi);
    await expect(pypi).toHaveAttribute("aria-pressed", "false");
  },
};

export const FormatsPressed: Story = {
  name: "Нажатые и усечённый счётчик",
  args: { title: "По форматам", items: FORMATS, initial: ["npm", "maven"] },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("button", { name: "npm 1 000+" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    await expect(canvas.getByRole("button", { name: "Cargo 17" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  },
};

export const ByWorkspace: Story = {
  name: "По workspace",
  args: { title: "По workspace", items: WORKSPACES, initial: ["billing"] },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: { title: "По форматам", items: FORMATS, initial: ["pypi"] },
};
