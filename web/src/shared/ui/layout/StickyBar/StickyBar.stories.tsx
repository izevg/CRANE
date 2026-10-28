import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, waitFor, within } from "storybook/test";
import { StickyBar } from "~/shared/ui/layout";
import { Button } from "~/shared/ui/primitives";

const meta = {
  title: "Раскладка/StickyBar",
  component: StickyBar<string>,
  args: { value: "Шаг 2 из 8", returnFocus: () => null, children: () => null },
  decorators: [
    (Story) => (
      <div className="h-72 w-[48rem] max-w-full overflow-auto rounded-md border border-border">
        <div className="h-96 p-4 text-muted-foreground">Содержимое карточки шага</div>
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof StickyBar<string>>;

export default meta;
type Story = StoryObj<typeof meta>;

export const WizardFooter: Story = {
  name: "Подвал мастера: «Назад · Сохранить и продолжить»",
  render: (args) => (
    <StickyBar {...args}>
      {(text) => (
        <div className="flex flex-wrap items-center gap-3">
          <span className="mr-auto">{text}</span>
          <Button variant="ghost" onClick={fn()}>
            Назад
          </Button>
          <Button onClick={fn()}>Сохранить и продолжить</Button>
        </div>
      )}
    </StickyBar>
  ),
  play: async ({ canvasElement }) => {
    const next = await within(canvasElement).findByRole("button", {
      name: "Сохранить и продолжить",
    });
    // Панель выезжает: пока идёт переход, она прозрачна
    await waitFor(() => expect(next).toBeVisible());
  },
};
