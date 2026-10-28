import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { ReadonlyValue } from "~/shared/ui/inputs";
import { Badge } from "~/shared/ui/primitives";

const HINT =
  "Задано переменной CRANE_DATA_DIR. Чтобы менять значение здесь, уберите её из окружения и перезапустите CRANE.";

const meta = {
  title: "Поля/ReadonlyValue",
  component: ReadonlyValue,
  args: { label: "Каталог данных", value: "/var/lib/crane" },
  decorators: [
    (Story) => (
      <div className="w-[28rem] max-w-full">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof ReadonlyValue>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Env: Story = {
  name: "Из окружения: замок и подсказка",
  args: { mono: true, lockHint: HINT },
  play: async ({ canvasElement }) => {
    const group = within(canvasElement).getByRole("group", { name: "Каталог данных" });
    await expect(group).toHaveTextContent("/var/lib/crane");
    await expect(group).toHaveAccessibleDescription(HINT);
    await expect(within(canvasElement).queryByRole("textbox")).toBeNull();
  },
};

export const EnvSecret: Story = {
  name: "Секрет из окружения: только состояние",
  args: {
    label: "Пароль базы данных",
    value: <Badge variant="outline">Задан</Badge>,
    lockHint: "Задано переменной CRANE_DB_PASSWORD. Значение не показывается.",
  },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("group", { name: "Пароль базы данных" }),
    ).toHaveTextContent("Задан");
  },
};

export const Plain: Story = {
  name: "Без замка",
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("group", { name: "Каталог данных" }),
    ).toHaveAccessibleDescription("");
  },
};

export const English: Story = {
  name: "EN",
  globals: { locale: "en" },
  args: {
    mono: true,
    lockHint: "Set by CRANE_DATA_DIR. To manage it here, remove the variable and restart CRANE.",
  },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("group", { name: "Каталог данных" }),
    ).toHaveTextContent("/var/lib/crane");
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: { mono: true, lockHint: HINT },
};
