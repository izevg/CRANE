import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { NotificationsBell } from "~/widgets/notifications-center";

const meta = {
  title: "Шелл/Уведомления/Колокольчик",
  component: NotificationsBell,
  args: { unread: 3 },
} satisfies Meta<typeof NotificationsBell>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Empty: Story = {
  name: "Нет непрочитанных",
  args: { unread: 0 },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("button", { name: "Уведомления" })).toBeVisible();
  },
};

export const Three: Story = {
  name: "3 непрочитанных",
  play: async ({ canvasElement }) => {
    const bell = within(canvasElement).getByRole("button", {
      name: "Уведомления, 3 непрочитанных",
    });
    await expect(bell).toHaveTextContent("3");
  },
};

export const Overflow: Story = {
  name: "12: значок «9+»",
  args: { unread: 12 },
  parameters: {
    a11y: {
      // Причина: patterns §28 требует видимое «9+» и точное число в имени кнопки; «9+» в имя не входит (WCAG 2.5.3, label-content-name-mismatch). Конфликт спецификации, решает UI-лид; компонент не менять до решения.
      config: { rules: [{ id: "label-content-name-mismatch", enabled: false }] },
    },
  },
  play: async ({ canvasElement }) => {
    const bell = within(canvasElement).getByRole("button", {
      name: "Уведомления, 12 непрочитанных",
    });
    await expect(bell).toHaveTextContent("9+");
  },
};
