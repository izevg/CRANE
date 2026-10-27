import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { ScoreBar } from "~/shared/ui/metrics";

const meta = {
  title: "Метрики/ScoreBar",
  component: ScoreBar,
  args: {
    label: "Релевантность эксплуатации",
    value: 0.88,
    tone: "error",
    confidence: 0.92,
    model: "systemone v3.1",
    factors: ["есть публичный эксплойт", "доступен из сети"],
  },
  decorators: [
    (Story) => (
      <div className="w-72">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof ScoreBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const High: Story = {
  name: "Высокая оценка: error",
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("meter")).toHaveAttribute(
      "aria-valuetext",
      "0,88, уверенность 0,92",
    );
  },
};

export const Medium: Story = {
  name: "Средняя оценка: warning",
  args: { label: "Радиус поражения", value: 0.62, tone: "warning", confidence: 0.81 },
};

export const Low: Story = {
  name: "Низкая оценка: info",
  args: { label: "Вирусность лицензии", value: 0.21, tone: "info", confidence: 0.77 },
};

export const LowConfidence: Story = {
  name: "Низкая уверенность: нейтральная полоса и пояснение",
  args: {
    value: 0.74,
    tone: "neutral",
    confidence: 0.38,
    notes: ["низкая уверенность, не влияет на политику"],
  },
  play: async ({ canvasElement }) => {
    await expect(
      within(canvasElement).getByText("низкая уверенность, не влияет на политику"),
    ).toBeInTheDocument();
  },
};

export const Shadow: Story = {
  name: "Shadow-модель: вторичная",
  args: {
    tone: "neutral",
    secondary: true,
    badge: "shadow",
    notes: ["не влияет на решения"],
    model: "systemone v3.2-rc",
  },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("shadow")).toBeInTheDocument();
    await expect(canvas.getByText("не влияет на решения")).toBeInTheDocument();
  },
};

export const Minimal: Story = {
  name: "Без уверенности, факторов и модели",
  render: () => <ScoreBar label="Радиус поражения" value={0.62} tone="warning" />,
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
};
