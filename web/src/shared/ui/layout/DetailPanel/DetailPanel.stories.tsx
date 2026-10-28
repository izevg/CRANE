import type { Meta, StoryObj } from "@storybook/react-vite";
import { type ComponentProps, useCallback, useRef, useState } from "react";
import { expect, fn, userEvent, waitFor, within } from "storybook/test";
import { DetailPanel } from "~/shared/ui/layout";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";

const VIEWPORTS = {
  wide: { name: "≥ 1536 (docked)", styles: { width: "1600px", height: "900px" } },
  narrow: { name: "Узкий (Sheet)", styles: { width: "1024px", height: "800px" } },
};

const meta = {
  title: "Раскладка/DetailPanel",
  component: DetailPanel,
  parameters: { layout: "fullscreen", viewport: { options: VIEWPORTS } },
  args: {
    open: true,
    onOpenChange: fn(),
    title: "lodash@4.17.20",
    status: <StatusBadge tone="warning" label="Карантин" />,
    children: null,
    menu: [{ label: "Открыть полностью", run: { href: "/packages/lodash/4.17.20" } }],
    tabs: (
      <Tabs defaultValue="overview">
        <TabsList variant="line" aria-label="Разделы версии">
          <TabsTrigger value="overview">Обзор</TabsTrigger>
          <TabsTrigger value="findings">Находки 2</TabsTrigger>
          <TabsTrigger value="evidence">Evidence</TabsTrigger>
        </TabsList>
        <TabsContent value="overview" />
      </Tabs>
    ),
    footer: {
      primary: { label: "Снять карантин", run: { onSelect: fn() } },
      secondary: [{ label: "Продлить проверку", run: { onSelect: fn() } }],
      menu: [{ label: "Заблокировать…", run: { onSelect: fn() }, destructive: true }],
    },
  },
} satisfies Meta<typeof DetailPanel>;

export default meta;
type Story = StoryObj<typeof meta>;

/** Таблица слева и панель справа; строка таблицы — цель возврата фокуса. */
function WithTable(args: ComponentProps<typeof DetailPanel>) {
  const [open, setOpen] = useState(args.open);
  const rowRef = useRef<HTMLButtonElement>(null);
  const show = useCallback(() => setOpen(true), []);
  return (
    <div className="flex items-start gap-6 p-6">
      <div className="min-w-0 flex-1">
        <h1 className="text-h1">Версии lodash</h1>
        <button
          type="button"
          ref={rowRef}
          onClick={show}
          className="mt-4 rounded-md border px-3 py-2"
        >
          Строка 4.17.20
        </button>
      </div>
      <DetailPanel {...args} open={open} onOpenChange={setOpen} returnFocusRef={rowRef}>
        <dl className="grid grid-cols-[auto_1fr] gap-x-4 gap-y-2 text-body">
          <dt className="text-muted-foreground">Основание</dt>
          <dd>CVE-2026-31337</dd>
          <dt className="text-muted-foreground">Потребителей</dt>
          <dd className="tabular-nums">63</dd>
        </dl>
      </DetailPanel>
    </div>
  );
}

export const Docked: Story = {
  name: "Docked (≥ 1536 px): немодальная область",
  // Реальная ширина окна нужна, потому что режим выбирает matchMedia, а не контейнер.
  globals: { viewport: { value: "wide", isRotated: false } },
  render: (args) => <WithTable {...args} />,
  play: async ({ canvasElement }) => {
    const panel = await within(canvasElement).findByRole("complementary", {
      name: "lodash@4.17.20",
    });
    await expect(within(document.body).queryByRole("dialog")).toBeNull();
    await expect(within(panel).getByRole("heading", { level: 2 })).toHaveFocus();
    await expect(within(panel).getByRole("button", { name: "Снять карантин" })).toBeVisible();
  },
};

export const DockedEscReturnsFocus: Story = {
  name: "Docked: Esc закрывает, фокус на строке",
  globals: { viewport: { value: "wide", isRotated: false } },
  render: (args) => <WithTable {...args} />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await canvas.findByRole("complementary");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(canvas.queryByRole("complementary")).toBeNull());
    await expect(canvas.getByRole("button", { name: "Строка 4.17.20" })).toHaveFocus();
  },
};

/** Остаётся открытым в момент axe-проверки. */
export const SheetNarrow: Story = {
  name: "Sheet (< 1536 px): модальный диалог",
  globals: { viewport: { value: "narrow", isRotated: false } },
  render: (args) => <WithTable {...args} />,
  play: async () => {
    const dialog = await within(document.body).findByRole("dialog", { name: "lodash@4.17.20" });
    await waitFor(() =>
      expect(
        within(dialog).getByRole("heading", { level: 2, name: "lodash@4.17.20" }),
      ).toHaveFocus(),
    );
    await expect(within(dialog).getByRole("button", { name: "Закрыть" })).toBeVisible();
    await expect(within(dialog).getByRole("tab", { name: "Находки 2" })).toBeVisible();
  },
};

export const SheetEscReturnsFocus: Story = {
  name: "Sheet: Esc закрывает, фокус на строке",
  globals: { viewport: { value: "narrow", isRotated: false } },
  render: (args) => <WithTable {...args} />,
  play: async ({ canvasElement }) => {
    await within(document.body).findByRole("dialog");
    await userEvent.keyboard("{Escape}");
    await waitFor(() => expect(within(document.body).queryByRole("dialog")).toBeNull());
    await expect(
      within(canvasElement).getByRole("button", { name: "Строка 4.17.20" }),
    ).toHaveFocus();
  },
};

export const WithoutFooter: Story = {
  name: "Без действий в подвале",
  globals: { viewport: { value: "wide", isRotated: false } },
  args: { footer: {}, menu: [], tabs: null },
  render: (args) => <WithTable {...args} />,
  play: async ({ canvasElement }) => {
    const panel = await within(canvasElement).findByRole("complementary");
    await expect(within(panel).getAllByRole("button")).toHaveLength(1);
  },
};
