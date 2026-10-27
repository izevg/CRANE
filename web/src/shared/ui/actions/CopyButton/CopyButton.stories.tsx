import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, spyOn, userEvent, waitFor, within } from "storybook/test";
import { CopyButton } from "~/shared/ui/actions";

const meta = {
  title: "Действия/CopyButton",
  component: CopyButton,
  // Буфер Chromium в headless недоступен без разрешений: подменяем запись, проверяем поведение кнопки.
  beforeEach: () => {
    spyOn(navigator.clipboard, "writeText").mockResolvedValue();
  },
  args: { value: "npm i @acme/ui@1.4.2", label: "Скопировать команду установки" },
} satisfies Meta<typeof CopyButton>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const button = within(canvasElement).getByRole("button", {
      name: "Скопировать команду установки",
    });
    await userEvent.click(button);
    await expect(navigator.clipboard.writeText).toHaveBeenCalledWith("npm i @acme/ui@1.4.2");
    await waitFor(() => expect(button).toHaveAttribute("data-state", "copied"));
    await expect(await within(canvasElement).findByText("Скопировано")).toBeInTheDocument();
  },
};
