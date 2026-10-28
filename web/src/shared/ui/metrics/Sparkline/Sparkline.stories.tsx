import type { Meta, StoryObj } from "@storybook/react-vite";
import { Sparkline } from "~/shared/ui/metrics";

const HIT_RATE = [93.1, 93.8, 94.4, 93.9, 94.6, 95.0, 94.2, 94.8, 94.2];

const meta = {
  title: "Метрики/Sparkline",
  component: Sparkline,
  args: { values: HIT_RATE },
  decorators: [
    (Story) => (
      <div className="h-12 w-64">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof Sparkline>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Neutral: Story = { name: "Нейтральный" };

export const WithReference: Story = {
  name: "С линией цели",
  args: { reference: 94 },
};

export const Warning: Story = { name: "Тон warning", args: { tone: "warning" } };

export const ErrorTone: Story = { name: "Тон error", args: { tone: "error" } };

export const NoData: Story = { name: "Нет данных: пунктир", args: { values: [] } };
