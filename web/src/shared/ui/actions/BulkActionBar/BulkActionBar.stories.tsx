import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, within } from "storybook/test";
import { type Action, BulkActionBar } from "~/shared/ui/actions";

const ACTIONS: Action[] = [
  {
    label: "Принять VEX",
    run: { onSelect: fn() },
    disabledReason: "VEX выдаётся на один бюллетень",
  },
  { label: "Исключение с TTL", run: { onSelect: fn() }, stepUp: true },
  { label: "Создать задачи", run: { onSelect: fn() } },
];

const meta = {
  title: "Действия/BulkActionBar",
  component: BulkActionBar,
  args: { count: 3, actions: ACTIONS, onClear: fn() },
  decorators: [
    (Story) => (
      <div className="h-64 w-[56rem] max-w-full overflow-auto">
        <div className="h-24 text-muted-foreground">Строки таблицы находок</div>
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof BulkActionBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Selected: Story = {
  name: "Выбрано 3",
  play: async ({ canvasElement, args }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("group", { name: "Массовые действия" })).toHaveTextContent(
      "Выбрано 3",
    );
    await expect(canvas.getByRole("button", { name: "Принять VEX" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
    await userEvent.click(canvas.getByRole("button", { name: "Снять выделение" }));
    await expect(args.onClear).toHaveBeenCalledTimes(1);
  },
};

export const DisabledReason: Story = {
  name: "Недоступное действие: причина читается",
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("button", { name: "Принять VEX" }),
    ).toHaveAccessibleDescription("VEX выдаётся на один бюллетень");
  },
};

export const StepUp: Story = {
  name: "Step-up: бейдж MFA",
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("MFA")).toBeInTheDocument();
  },
};

export const OneAction: Story = {
  name: "Одно действие",
  args: { count: 1, actions: [ACTIONS[2] as Action] },
};

export const Empty: Story = {
  name: "Ничего не выбрано: панели нет",
  args: { count: 0 },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole("group")).toBeNull();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
};
