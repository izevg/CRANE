import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import { ActionButton } from "~/shared/ui/actions";

const onSelect = fn();
const blockedSelect = fn();
const STEP_UP_HINT = /требуется подтверждение входа/;

const meta = {
  title: "Действия/ActionButton",
  component: ActionButton,
  args: {
    variant: "primary",
    action: { label: "Скопировать команду установки", run: { onSelect } },
  },
} satisfies Meta<typeof ActionButton>;

export default meta;
type Story = StoryObj<typeof meta>;

export const OnSelect: Story = {
  name: "onSelect: кнопка",
  play: async ({ canvasElement, args }) => {
    if (!("onSelect" in args.action.run)) {
      throw new Error("Ожидалось действие с onSelect");
    }
    await userEvent.click(
      within(canvasElement).getByRole("button", { name: "Скопировать команду установки" }),
    );
    await expect(args.action.run.onSelect).toHaveBeenCalledTimes(1);
  },
};

export const Href: Story = {
  name: "href: ссылка",
  args: {
    variant: "secondary",
    action: { label: "Открыть пакет", run: { href: "/packages/acme-ui" } },
  },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByRole("link", { name: "Открыть пакет" }),
    ).toHaveAttribute("href", "/packages/acme-ui");
  },
};

export const Variants: Story = {
  render: () => (
    <div className="flex flex-wrap items-center gap-3">
      <ActionButton variant="primary" action={{ label: "Опубликовать", run: { onSelect: fn() } }} />
      <ActionButton
        variant="secondary"
        action={{ label: "В избранное", run: { onSelect: fn() } }}
      />
      <ActionButton
        variant="ghost"
        action={{ label: "Открыть ↗", run: { href: "/packages/acme-ui" } }}
      />
      <ActionButton
        variant="primary"
        action={{ label: "Заблокировать версию", run: { onSelect: fn() }, destructive: true }}
      />
    </div>
  ),
};

export const DisabledWithReason: Story = {
  name: "Недоступно: причина читается и с клавиатуры",
  args: {
    action: {
      label: "Снять блок",
      run: { onSelect: blockedSelect },
      disabledReason: "Нужна роль AppSec",
    },
  },
  play: async ({ canvasElement }) => {
    const button = within(canvasElement).getByRole("button", { name: "Снять блок" });
    await expect(button).toHaveAttribute("aria-disabled", "true");
    await expect(button).toHaveAccessibleDescription("Нужна роль AppSec");
    await userEvent.tab();
    await expect(button).toHaveFocus();
    await waitFor(async () =>
      expect(await within(document.body).findAllByText("Нужна роль AppSec")).toHaveLength(2),
    );
    await userEvent.click(button);
    await expect(blockedSelect).not.toHaveBeenCalled();
  },
};

export const StepUp: Story = {
  name: "Step-up: бейдж MFA и пояснение в имени",
  args: { action: { label: "Применить retention", run: { onSelect: fn() }, stepUp: true } },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("MFA")).toBeVisible();
    await expect(canvas.getByRole("button")).toHaveAccessibleName(STEP_UP_HINT);
  },
};
