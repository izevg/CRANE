import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { StatChip } from "~/shared/ui/metrics";

const meta = {
  title: "Метрики/StatChip",
  component: StatChip,
  args: { label: "Версий", value: "48" },
} satisfies Meta<typeof StatChip>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  name: "Один чип",
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).queryByRole("link")).toBeNull();
  },
};

function Row() {
  return (
    <div className="flex flex-wrap items-center gap-2">
      <StatChip label="Версий" value="48" />
      <StatChip label="Последняя" value="1.4.2" />
      <StatChip label="Загрузок за 30 дн" value="12 340" />
    </div>
  );
}

export const HeaderRow: Story = {
  name: "Ряд чипов в шапке страницы",
  render: () => <Row />,
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  render: () => <Row />,
};
