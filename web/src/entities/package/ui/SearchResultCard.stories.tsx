import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, within } from "storybook/test";
import { SearchResultCard } from "../index";
import { BASE_ITEM } from "./fixtures";

const meta = {
  title: "Сущности/Пакет/Карточка результата",
  component: SearchResultCard,
  args: { item: BASE_ITEM, href: "/packages/acme-ui" },
  decorators: [
    (Story) => (
      <div className="max-w-3xl">
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof SearchResultCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  name: "Обычная: без пилюли риска",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("link", { name: "@acme/ui" })).toHaveAttribute(
      "href",
      "/packages/acme-ui",
    );
    await expect(canvas.getAllByRole("link")).toHaveLength(1);
  },
};

export const ExactMatch: Story = {
  name: "Точное совпадение",
  args: { item: { ...BASE_ITEM, exact_match: true } },
};

export const Typosquat: Story = {
  name: "Похож на другой пакет (typosquat)",
  args: {
    item: {
      ...BASE_ITEM,
      name: { display: "@acme/uii", normalized: "@acme/uii" },
      repository: { name: "npm-proxy", format: "npm", kind: "proxy", workspace: "platform" },
      latest: { version: "0.0.1", state: "quarantined" },
      typosquat: { similar_to: "@acme/ui", distance: 1 },
    },
    typosquatHref: "/packages/acme-ui",
  },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText(SQUAT)).toBeVisible();
  },
};

export const WithRisk: Story = {
  name: "С риском: P1 · 1 находка",
  args: { item: { ...BASE_ITEM, risk: { max_open_priority: "P1", open_count: 1 } } },
  play: async ({ canvasElement }) => {
    await expect(within(canvasElement).getByText("P1 · 1 находка")).toBeVisible();
  },
};

export const Deprecated: Story = {
  name: "Устаревший",
  args: {
    item: {
      ...BASE_ITEM,
      lifecycle: { state: "deprecated" },
      risk: { max_open_priority: "P3", open_count: 3 },
    },
  },
};

export const LongName: Story = {
  name: "Длинное имя на 390 px переносится",
  args: {
    item: {
      ...BASE_ITEM,
      name: {
        display: "com.fasterxml.jackson.core:jackson-databind-extra-long-artifact-name",
        normalized: "com.fasterxml.jackson.core:jackson-databind-extra-long-artifact-name",
      },
      format: "maven",
      risk: { max_open_priority: "P0", open_count: 12 },
      typosquat: { similar_to: "com.fasterxml.jackson.core:jackson-databind", distance: 2 },
    },
  },
  decorators: [
    (Story) => (
      <div className="w-[390px]">
        <Story />
      </div>
    ),
  ],
  play: async ({ canvasElement }) => {
    await expect(document.documentElement.scrollWidth).toBeLessThanOrEqual(
      document.documentElement.clientWidth,
    );
    await expect(within(canvasElement).getByRole("article")).toBeVisible();
  },
};

export const English: Story = {
  name: "EN: подписи на английском",
  args: {
    item: { ...BASE_ITEM, exact_match: true, risk: { max_open_priority: "P1", open_count: 2 } },
  },
  globals: { locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Exact match")).toBeVisible();
    await expect(canvas.getByText("P1 · 2 findings")).toBeVisible();
    await expect(canvas.getByText(DOWNLOADS_EN)).toBeVisible();
  },
};

export const Dark: Story = {
  name: "Тёмная тема",
  globals: { theme: "dark" },
  args: {
    item: { ...BASE_ITEM, exact_match: true, risk: { max_open_priority: "P1", open_count: 1 } },
  },
};

const SQUAT = /Похоже на @acme\/ui — 1 символ/;
const DOWNLOADS_EN = /12,340 downloads in 30 d/;
