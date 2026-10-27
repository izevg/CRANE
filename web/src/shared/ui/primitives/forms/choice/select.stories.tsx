import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "~/shared/ui/primitives";

const WORKSPACES = [
  { value: "all", label: "Workspace: все" },
  { value: "platform", label: "platform" },
  { value: "billing", label: "billing" },
  { value: "mobile", label: "mobile" },
];

const meta = {
  title: "Примитивы/Select",
  component: Select,
  render: () => (
    <Select items={WORKSPACES} defaultValue="all">
      <SelectTrigger aria-label="Workspace">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          <SelectLabel>Workspace</SelectLabel>
          {WORKSPACES.map((item) => (
            <SelectItem key={item.value} value={item.value}>
              {item.label}
            </SelectItem>
          ))}
        </SelectGroup>
      </SelectContent>
    </Select>
  ),
} satisfies Meta<typeof Select>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("combobox", { name: "Workspace" }));
    await userEvent.click(await within(document.body).findByRole("option", { name: "billing" }));
    await expect(
      within(canvasElement).getByRole("combobox", { name: "Workspace" }),
    ).toHaveTextContent("billing");
    await waitFor(() => expect(within(document.body).queryByRole("listbox")).toBeNull());
  },
};
