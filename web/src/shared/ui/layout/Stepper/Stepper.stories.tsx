import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, within } from "storybook/test";
import { Stepper, type StepperStep } from "~/shared/ui/layout";

const VIEWPORTS = {
  wide: { name: "Широкий (рейл)", styles: { width: "1280px", height: "800px" } },
  narrow: { name: "Узкий (< lg)", styles: { width: "390px", height: "844px" } },
};
const WIDE = { viewport: { value: "wide", isRotated: false } } as const;
const NARROW = { viewport: { value: "narrow", isRotated: false } } as const;

const STEPS: StepperStep[] = [
  { id: "storage", label: "Хранилище", status: "done_by_env", href: "/setup?step=storage" },
  { id: "database", label: "База данных", status: "done", href: "/setup?step=database" },
  { id: "auth", label: "Учётная запись", status: "todo", href: "/setup?step=auth" },
  { id: "network", label: "Сеть и адреса", status: "todo" },
  { id: "oidc", label: "Единый вход", status: "unavailable", href: "/setup?step=oidc" },
];

const meta = {
  title: "Раскладка/Stepper",
  component: Stepper,
  parameters: { viewport: { options: VIEWPORTS } },
  globals: WIDE,
  args: { label: "Шаги настройки", steps: STEPS, current: "auth" },
  decorators: [
    (Story) => (
      <div className="w-64 max-w-full">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof Stepper>;

export default meta;
type Story = StoryObj<typeof meta>;

export const AllStatuses: Story = {
  name: "Все статусы, текущий — «Учётная запись»",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("link", { current: "step" })).toHaveTextContent("Учётная запись");
    await expect(
      canvas.getByRole("link", { name: "Хранилище, выполнено окружением" }),
    ).toBeVisible();
    await expect(canvas.getByRole("link", { name: "База данных, выполнено" })).toBeVisible();
    await expect(canvas.getByRole("link", { name: "Единый вход, недоступно" })).toBeVisible();
    await expect(canvas.queryByRole("link", { name: "Сеть и адреса, не выполнено" })).toBeNull();
  },
};

export const Narrow: Story = {
  name: "Узкий экран: «Шаг n из N» и «Все шаги»",
  globals: NARROW,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const toggle = canvas.getByRole("button", { name: "Все шаги" });
    await expect(canvas.getByText("Шаг 3 из 5 · Учётная запись")).toBeVisible();
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
    await userEvent.click(toggle);
    await expect(toggle).toHaveAttribute("aria-expanded", "true");
    await expect(canvas.getAllByRole("listitem")).toHaveLength(STEPS.length);
    await userEvent.click(toggle);
    await expect(toggle).toHaveAttribute("aria-expanded", "false");
  },
};

export const NarrowOpen: Story = {
  name: "Узкий экран: список раскрыт (axe)",
  globals: NARROW,
  play: async ({ canvasElement }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Все шаги" }));
    await expect(within(canvasElement).getByRole("list")).toBeVisible();
  },
};

export const English: Story = {
  name: "EN",
  globals: { ...NARROW, locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Step 3 of 5 · Учётная запись")).toBeVisible();
    await userEvent.click(canvas.getByRole("button", { name: "All steps" }));
    await expect(canvas.getByRole("link", { name: "Хранилище, set by environment" })).toBeVisible();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { ...WIDE, theme: "dark" },
};
