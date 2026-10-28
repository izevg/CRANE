import type { Meta, StoryObj } from "@storybook/react-vite";
import { useCallback, useState } from "react";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import { StickySaveBar } from "~/shared/ui/actions";

const formTarget = () => document.querySelector<HTMLElement>("[data-focus-target]");

const meta = {
  title: "Действия/StickySaveBar",
  component: StickySaveBar,
  args: { dirty: 3, state: "idle", onSave: fn(), onDiscard: fn(), returnFocus: formTarget },
  decorators: [
    (Story) => (
      <div className="h-72 w-[48rem] max-w-full overflow-auto rounded-md border border-border">
        <form data-focus-target tabIndex={-1} className="h-96 p-4 text-muted-foreground">
          Поля формы настроек
        </form>
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof StickySaveBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Hidden: Story = {
  name: "Нет правок: панели нет",
  args: { dirty: 0, state: "idle" },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole("button")).toBeNull();
  },
};

export const Dirty: Story = {
  name: "Изменено 3",
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    // Панель выезжает: пока идёт переход, она прозрачна
    await waitFor(() => expect(canvas.getAllByText("Изменено: 3")[0]).toBeVisible());
    await userEvent.click(canvas.getByRole("button", { name: "Сохранить" }));
    await expect(args.onSave).toHaveBeenCalledTimes(1);
    await userEvent.click(canvas.getByRole("button", { name: "Отменить" }));
    await expect(args.onDiscard).toHaveBeenCalledTimes(1);
  },
};

export const Saving: Story = {
  name: "Сохранение: кнопки заблокированы",
  args: { state: "saving" },
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    const save = canvas.getByRole("button", { name: "Сохранить" });
    await expect(save).toHaveAttribute("aria-busy", "true");
    await expect(save).toHaveAttribute("aria-disabled", "true");
    await expect(canvas.getByRole("button", { name: "Отменить" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
    await userEvent.click(save);
    await expect(args.onSave).not.toHaveBeenCalled();
  },
};

export const Partial: Story = {
  name: "Частичный итог: сохранено 2 из 3",
  args: { dirty: 1, state: { saved: 2, total: 3 } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await waitFor(() => expect(canvas.getAllByText("Сохранено 2 из 3")[0]).toBeVisible());
    await expect(canvas.getByText("Изменено: 1")).toBeVisible();
  },
};

export const FocusReturn: Story = {
  name: "«Отменить»: панель уходит, фокус на форме",
  render: function Render(args) {
    const [dirty, setDirty] = useState(2);
    const discard = useCallback(() => setDirty(0), []);
    return <StickySaveBar {...args} dirty={dirty} onDiscard={discard} />;
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(await canvas.findByRole("button", { name: "Отменить" }));
    await waitFor(() => expect(canvas.queryByRole("button", { name: "Сохранить" })).toBeNull());
    await expect(document.activeElement).toBe(formTarget());
  },
};

export const English: Story = {
  name: "EN",
  globals: { locale: "en" },
  args: { dirty: 1, state: { saved: 2, total: 3 } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await waitFor(() => expect(canvas.getAllByText("1 changed")[0]).toBeVisible());
    await expect(canvas.getAllByText("Saved 2 of 3")[0]).toBeVisible();
    await expect(canvas.getByRole("button", { name: "Discard" })).toBeVisible();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
};
