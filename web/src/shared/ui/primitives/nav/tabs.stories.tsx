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
