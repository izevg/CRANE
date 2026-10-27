import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
import { LocaleMenu } from "./LocaleMenu";

const meta = {
  title: "Шелл/Переключатель языка",
  component: LocaleMenu,
  parameters: { layout: "centered" },
  args: { collapsed: false },
  decorators: [
    (Story) => (
      <div className="w-60 bg-sidebar p-3 text-sidebar-foreground">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof LocaleMenu>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Остаётся открытой в момент axe-проверки. */
export const Open: Story = {
  name: "Меню открыто",
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Язык: Русский" }));
    const body = within(document.body);
    await expect(await body.findByRole("menuitemradio", { name: "Русский" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
  },
};

export const Rail: Story = {
  name: "Рельса: только значок",
  args: { collapsed: true },
};

export const SwitchToEnglish: Story = {
  name: "Выбор English меняет lang и подписи",
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Язык: Русский" }));
    await userEvent.click(
      await within(document.body).findByRole("menuitemradio", { name: "English" }),
    );
    await waitFor(() => expect(document.documentElement.lang).toBe("en"));
    await expect(
      await within(canvasElement).findByRole("button", { name: "Language: English" }),
    ).toBeVisible();
    window.localStorage.removeItem("crane.locale");
  },
};
