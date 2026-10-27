import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { ShareBar } from "~/shared/ui/metrics";

const TEAMS = [
  { id: "platform", label: "Платформа", value: 5.1, display: "5,1 ТБ" },
  { id: "payments", label: "Платежи", value: 3.8, display: "3,8 ТБ" },
  { id: "data", label: "Данные", value: 2.9, display: "2,9 ТБ" },
  { id: "mobile", label: "Мобильная", value: 2.2, display: "2,2 ТБ" },
  { id: "web", label: "Веб", value: 1.6, display: "1,6 ТБ" },
  { id: "ml", label: "ML", value: 1.1, display: "1,1 ТБ" },
  { id: "infra", label: "Инфраструктура", value: 0.8, display: "0,8 ТБ" },
  { id: "security", label: "Безопасность", value: 0.5, display: "0,5 ТБ" },
  { id: "docs", label: "Документация", value: 0.3, display: "0,3 ТБ" },
  { id: "labs", label: "Лаборатория", value: 0.1, display: "0,1 ТБ" },
];

const OTHER_ROW = /Прочие \(2\)/;
const formatTb = (sum: number) => `${sum.toFixed(1).replace(".", ",")} ТБ`;

const meta = {
  title: "Метрики/ShareBar",
  component: ShareBar,
  args: {
    title: "Объём по командам",
    columns: { name: "Команда", value: "Объём", share: "Доля" },
    items: TEAMS,
    formatOther: formatTb,
    caption: "Дедупликация сэкономила 4,2 ТБ",
  },
  decorators: [
    (Story) => (
      <div className="w-[28rem]">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof ShareBar>;

export default meta;
type Story = StoryObj<typeof meta>;

export const TopEightAndOther: Story = {
  name: "10 команд: 8 + «Прочие (2)»",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("row", { name: OTHER_ROW })).toBeInTheDocument();
    await expect(canvas.getAllByRole("row")).toHaveLength(10);
  },
};

export const FewItems: Story = {
  name: "Три команды без свёртки",
  args: { items: TEAMS.slice(0, 3), caption: undefined },
};

export const ByFormat: Story = {
  name: "Колонки по формату",
  args: {
    title: "Объём по форматам",
    columns: { name: "Формат", value: "Объём", share: "Доля" },
    items: [
      { id: "npm", label: "npm", value: 9.2, display: "9,2 ТБ" },
      { id: "oci", label: "OCI", value: 6.1, display: "6,1 ТБ" },
      { id: "maven", label: "Maven", value: 3.1, display: "3,1 ТБ" },
    ],
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
};

const OTHER_EN = /Other \(2\)/;

export const English: Story = {
  name: "EN: подписи на английском",
  globals: { locale: "en" },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByRole("row", { name: OTHER_EN })).toBeInTheDocument();
  },
};
