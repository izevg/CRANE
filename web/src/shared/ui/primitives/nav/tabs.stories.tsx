import type { Meta, StoryObj } from "@storybook/react-vite";
import { expect, userEvent, waitFor, within } from "storybook/test";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/shared/ui/primitives";

const meta = {
  title: "Примитивы/Tabs",
  component: Tabs,
  args: { defaultValue: "overview" },
} satisfies Meta<typeof Tabs>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  render: (args) => (
    <Tabs {...args}>
      <TabsList aria-label="Разделы пакета">
        <TabsTrigger value="overview">Обзор</TabsTrigger>
        <TabsTrigger value="versions">Версии 48</TabsTrigger>
        <TabsTrigger value="readme">README</TabsTrigger>
      </TabsList>
      <TabsContent value="overview">Описание пакета @acme/ui</TabsContent>
      <TabsContent value="versions">Список версий</TabsContent>
      <TabsContent value="readme">README пакета</TabsContent>
    </Tabs>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("tab", { name: "Обзор" }));
    await userEvent.keyboard("{ArrowRight}");
    await expect(canvas.getByRole("tab", { name: "Версии 48" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    // предыдущая панель уходит с transition и некоторое время ещё в DOM: ждём, пока останется одна
    await waitFor(() => expect(canvas.getByRole("tabpanel")).toHaveTextContent("Список версий"));
  },
};

export const Line: Story = {
  render: () => (
    <Tabs defaultValue="overview">
      <TabsList variant="line" aria-label="Разделы пакета">
        <TabsTrigger value="overview">Обзор</TabsTrigger>
        <TabsTrigger value="versions">Версии 48</TabsTrigger>
        <TabsTrigger value="risks">Риски 1</TabsTrigger>
      </TabsList>
      <TabsContent value="overview">Описание пакета @acme/ui</TabsContent>
      <TabsContent value="versions">Список версий</TabsContent>
      <TabsContent value="risks">P1 · 1 находка</TabsContent>
    </Tabs>
  ),
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await userEvent.click(canvas.getByRole("tab", { name: "Обзор" }));
    await userEvent.keyboard("{ArrowRight}{ArrowRight}");
    await expect(canvas.getByRole("tab", { name: "Риски 1" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
  },
};

const LAYOUT_OR_ALL = /width|height|all/;

/** Индикатор двигают translate и scale, а не width/height: Layout во время скольжения не запускается (spec 2). */
export const IndicatorMotion: Story = {
  name: "Индикатор line: transform вместо width/height",
  render: () => (
    <Tabs defaultValue="overview">
      <TabsList variant="line" aria-label="Разделы пакета">
        <TabsTrigger value="overview">Обзор</TabsTrigger>
        <TabsTrigger value="versions">Версии 48</TabsTrigger>
      </TabsList>
      <TabsContent value="overview">Описание</TabsContent>
      <TabsContent value="versions">Версии</TabsContent>
    </Tabs>
  ),
  play: async ({ canvasElement }) => {
    const indicator = canvasElement.querySelector("[data-slot=tabs-indicator]");
    if (!(indicator instanceof HTMLElement)) {
      throw new Error("нет индикатора");
    }
    const style = getComputedStyle(indicator);
    await expect(style.transitionProperty).not.toMatch(LAYOUT_OR_ALL);
    await expect(style.transitionProperty).toContain("translate");
    await expect(parseFloat(style.transformOrigin)).toBe(0);
    await userEvent.click(within(canvasElement).getByRole("tab", { name: "Версии 48" }));
    await expect(getComputedStyle(indicator).transitionProperty).not.toMatch(LAYOUT_OR_ALL);
  },
};

/** При reduced motion индикатор не скользит: переход нулевой длины, а не укороченный до 150 мс. */
export const IndicatorReducedMotion: Story = {
  ...IndicatorMotion,
  name: "Индикатор line: при reduced motion без перехода",
  play: async ({ canvasElement }) => {
    const indicator = canvasElement.querySelector("[data-slot=tabs-indicator]");
    if (!(indicator instanceof HTMLElement)) {
      throw new Error("нет индикатора");
    }
    document.documentElement.setAttribute("data-motion", "reduce");
    try {
      await expect(getComputedStyle(indicator).transitionDuration).toBe("0s");
    } finally {
      document.documentElement.removeAttribute("data-motion");
    }
  },
};
