import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, userEvent, waitFor, within } from "storybook/test";
import { FilterSelect } from "~/shared/ui/inputs";

const REPOSITORIES = [
  { value: "npm-internal", label: "npm-internal" },
  { value: "pypi-proxy", label: "pypi-proxy" },
  { value: "maven-releases", label: "maven-releases" },
];

const WORKSPACES = [
  { value: "platform", label: "platform" },
  { value: "billing", label: "billing" },
  { value: "mobile", label: "mobile" },
];

type Props = {
  label: string;
  allLabel: string;
  options: readonly { value: string; label: string }[];
  initial: string | null;
};

function Controlled({ label, allLabel, options, initial }: Props) {
  const [value, setValue] = useState<string | null>(initial);
  return (
    <FilterSelect
      label={label}
      allLabel={allLabel}
      options={options}
      value={value}
      onValueChange={setValue}
    />
  );
}

const meta = {
  title: "Фильтры/FilterSelect",
  component: Controlled,
} satisfies Meta<typeof Controlled>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Repositories: Story = {
  name: "Все репозитории",
  args: { label: "Репозиторий", allLabel: "Все репозитории", options: REPOSITORIES, initial: null },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const body = within(document.body);
    const trigger = canvas.getByRole("combobox", { name: "Репозиторий" });
    // Повторно открывать список можно только после того, как закрылся предыдущий (exit-анимация): иначе
    // `findByRole("option")` может попасть в закрывающийся список.
    const listClosed = () => waitFor(() => expect(body.queryByRole("listbox")).toBeNull());
    await waitFor(() => expect(trigger).toHaveTextContent("Все репозитории"));
    await userEvent.click(trigger);
    await userEvent.click(await body.findByRole("option", { name: "pypi-proxy" }));
    await waitFor(() => expect(trigger).toHaveTextContent("pypi-proxy"));
    await listClosed();
    await userEvent.click(trigger);
    await userEvent.click(await body.findByRole("option", { name: "Все репозитории" }));
    await waitFor(() => expect(trigger).toHaveTextContent("Все репозитории"));
    await listClosed();
  },
};

export const Workspace: Story = {
  name: "Workspace: все",
  args: { label: "Workspace", allLabel: "Workspace: все", options: WORKSPACES, initial: null },
};

export const Selected: Story = {
  name: "Выбранное значение",
  args: { label: "Workspace", allLabel: "Workspace: все", options: WORKSPACES, initial: "billing" },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("combobox", { name: "Workspace" }),
    ).toHaveTextContent("billing");
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: { label: "Workspace", allLabel: "Workspace: все", options: WORKSPACES, initial: "billing" },
};
