import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";
import { expect, fn, userEvent, within } from "storybook/test";
import { type Action, CopyableId } from "~/shared/ui/actions";
import { columnHelper, DataTable } from "~/shared/ui/data-display";
import {
  Badge,
  Callout,
  CalloutDescription,
  CalloutTitle,
  type Tone,
} from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";

type Version = {
  id: string;
  version: string;
  state: { tone: Tone; label: string };
  flags: readonly string[];
  downloads: number;
  by: string;
  published: string;
};

const VERSIONS: readonly Version[] = [
  {
    id: "v-142",
    version: "1.4.2",
    state: { tone: "warning", label: "Карантин" },
    flags: ["latest"],
    downloads: 214,
    by: "robot:ci-publisher",
    published: "27.10.2026, 13:42",
  },
  {
    id: "v-141",
    version: "1.4.1",
    state: { tone: "success", label: "Выпущена" },
    flags: ["latest", "stable"],
    downloads: 12340,
    by: "Анна Крылова",
    published: "21.10.2026, 09:15",
  },
  {
    id: "v-140",
    version: "1.4.0",
    state: { tone: "error", label: "Заблокирована" },
    flags: ["yanked"],
    downloads: 8120,
    by: "robot:ci-publisher",
    published: "14.10.2026, 17:03",
  },
  {
    id: "v-139",
    version: "1.3.9",
    state: { tone: "neutral", label: "Удалена" },
    flags: [],
    downloads: 640,
    by: "Игорь Седов",
    published: "02.10.2026, 11:30",
  },
  {
    id: "v-138",
    version: "1.3.8",
    state: { tone: "success", label: "Выпущена" },
    flags: ["deprecated"],
    downloads: 31870,
    by: "robot:ci-publisher",
    published: "18.09.2026, 08:47",
  },
];

const helper = columnHelper<Version>();
const columns = helper.columns([
  helper.accessor("version", {
    header: "Версия",
    meta: { priority: 1 },
    cell: (info) => <span className="font-semibold">{info.getValue()}</span>,
  }),
  helper.accessor("state", {
    header: "Состояние",
    meta: { priority: 1 },
    cell: (info) => <StatusBadge tone={info.getValue().tone} label={info.getValue().label} />,
  }),
  helper.accessor("flags", {
    header: "Флаги",
    meta: { priority: 2 },
    cell: (info) => (
      <span className="flex gap-1">
        {info.getValue().map((flag) => (
          <Badge key={flag} variant="outline">
            {flag}
          </Badge>
        ))}
      </span>
    ),
  }),
  helper.accessor("downloads", {
    header: "Загрузок",
    meta: { priority: 2, numeric: true },
    cell: (info) => new Intl.NumberFormat("ru-RU").format(info.getValue()),
  }),
  helper.accessor("by", { header: "Кем", meta: { priority: 3 } }),
  helper.accessor("published", { header: "Опубликована", meta: { priority: 3 } }),
]);

const getRowId = (row: Version) => row.id;

const meta = {
  title: "Данные/DataTable",
  component: DataTable<Version>,
  decorators: [
    (Story) => (
      <div className="@container/page">
        <Story />
      </div>
    ),
  ],
  args: { columns, data: [...VERSIONS], getRowId, caption: "Версии @acme/ui" },
} satisfies Meta<typeof DataTable<Version>>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("columnheader", { name: "Версия" })).toBeVisible();
    await expect(canvas.getByRole("table")).toHaveAttribute("aria-rowcount", "6");
  },
};

export const Loading: Story = { args: { loading: true, data: [] } };

export const Empty: Story = {
  args: {
    data: [],
    empty: <p className="p-6 text-center text-muted-foreground">Версий пока нет</p>,
  },
};

export const ErrorState: Story = {
  name: "Ошибка",
  args: {
    error: (
      <Callout tone="error" live="alert" className="m-3 w-auto">
        <CalloutTitle>Не удалось загрузить версии</CalloutTitle>
        <CalloutDescription>Повторите попытку позже. Код запроса req_0192f3a4.</CalloutDescription>
      </Callout>
    ),
  },
};

function SelectableTable({ compact = false }: { compact?: boolean }) {
  const [selectedId, setSelectedId] = useState<string | null>("v-141");
  return (
    <DataTable
      columns={columns}
      data={[...VERSIONS]}
      getRowId={getRowId}
      caption="Версии @acme/ui"
      compact={compact}
      selectedId={selectedId}
      onRowSelect={setSelectedId}
    />
  );
}

export const SelectedRow: Story = {
  name: "Выбранная строка, стрелки ↑/↓",
  render: () => <SelectableTable />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const selected = () => canvas.getByRole("row", { selected: true });
    await expect(selected()).toHaveTextContent("1.4.1");
    selected().focus();
    await userEvent.keyboard("{ArrowDown}");
    await expect(selected()).toHaveTextContent("1.4.0");
    await expect(selected()).toHaveFocus();
    await userEvent.keyboard("{ArrowUp}{ArrowUp}");
    await expect(selected()).toHaveTextContent("1.4.2");
  },
};

export const Compact: Story = {
  name: "Открыта панель деталей (compact)",
  render: () => <SelectableTable compact />,
};

const FINDINGS = [
  { id: "f-1", version: "lodash 4.17.20", state: VERSIONS[2]?.state, flags: [], downloads: 63 },
  { id: "f-2", version: "axios 0.21.1", state: VERSIONS[0]?.state, flags: [], downloads: 41 },
  { id: "f-3", version: "minimist 1.2.5", state: VERSIONS[1]?.state, flags: [], downloads: 17 },
] as const;

type Finding = (typeof FINDINGS)[number];
const getFindingId = (row: { id: string }) => row.id;
const findingHelper = columnHelper<Finding>();
const findingColumns = findingHelper.columns([
  findingHelper.accessor("version", { header: "Пакет", meta: { priority: 1 } }),
  findingHelper.accessor("downloads", {
    header: "Проектов",
    meta: { priority: 1, numeric: true },
  }),
]);

function SelectionTable() {
  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set());
  return (
    <DataTable
      columns={findingColumns}
      data={[...FINDINGS]}
      getRowId={getFindingId}
      caption="Находки"
      selection={{
        selected,
        onChange: setSelected,
        label: (row) => `Выбрать ${row.version}`,
      }}
    />
  );
}

export const WithSelection: Story = {
  name: "Выбор чекбоксами (находки)",
  render: () => <SelectionTable />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const second = canvas.getByRole("checkbox", { name: "Выбрать axios 0.21.1" });
    second.focus();
    await userEvent.keyboard(" ");
    await expect(second).toBeChecked();
    await userEvent.click(canvas.getByRole("checkbox", { name: "Выбрать все загруженные" }));
    await expect(canvas.getByRole("checkbox", { name: "Выбрать lodash 4.17.20" })).toBeChecked();
    await expect(canvas.getByRole("checkbox", { name: "Выбрать minimist 1.2.5" })).toBeChecked();
  },
};

const rowActions = (row: Version): readonly Action[] => [
  { label: "Открыть", run: { href: `/versions/${row.version}` } },
  { label: "Скачать SBOM", run: { onSelect: fn() } },
  { label: "Заблокировать", run: { onSelect: fn() }, destructive: true, stepUp: true },
];

export const WithActions: Story = {
  name: "Колонка действий",
  args: { actions: rowActions },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getAllByRole("button", { name: "Другие действия" })).toHaveLength(5);
  },
};

export const LoadMore: Story = {
  name: "Загрузить ещё",
  args: { loadMore: { label: "Загрузить ещё", run: { onSelect: fn() } } },
  play: async ({ canvasElement, args }) => {
    await userEvent.click(within(canvasElement).getByRole("button", { name: "Загрузить ещё" }));
    await expect(args.loadMore?.run).toHaveProperty("onSelect");
  },
};

const MANY: readonly Version[] = Array.from({ length: 400 }, (_, index) => ({
  id: `v-${index}`,
  version: `1.${Math.floor(index / 10)}.${index % 10}`,
  state: VERSIONS[index % VERSIONS.length]?.state ?? { tone: "neutral", label: "Удалена" },
  flags: [],
  downloads: 1000 + index * 7,
  by: index % 2 === 0 ? "robot:ci-publisher" : "Анна Крылова",
  published: "01.10.2026, 10:00",
}));

export const Virtualized: Story = {
  name: "Более 200 строк (виртуализация)",
  args: { data: [...MANY] },
  play: async ({ canvasElement }) => {
    const rows = within(canvasElement).getAllByRole("row");
    await expect(rows.length).toBeLessThan(MANY.length);
  },
};

const WIDE_COLUMNS = columnHelper<Version>().columns([
  helper.accessor("version", { header: "Версия", meta: { priority: 1 } }),
  helper.accessor("published", { header: "Опубликована", meta: { priority: 1 } }),
  helper.accessor("by", { header: "Кем", meta: { priority: 1 } }),
  helper.accessor("state", {
    header: "Состояние",
    meta: { priority: 1 },
    cell: (info) => <StatusBadge tone={info.getValue().tone} label={info.getValue().label} />,
  }),
  helper.accessor("downloads", {
    header: "Загрузок",
    meta: { priority: 1, numeric: true },
  }),
  helper.accessor("id", {
    header: "Идентификатор",
    meta: { priority: 1 },
    cell: (info) => <CopyableId value={`0192f3a4-7b1c-7e21-9d21-${info.getValue()}`} />,
  }),
]);

export const NarrowScroll: Story = {
  name: "Узкий контейнер: горизонтальная прокрутка, закреплённые колонки",
  args: { columns: WIDE_COLUMNS, actions: rowActions },
  decorators: [
    (Story) => (
      <div className="w-[420px]">
        <Story />
      </div>
    ),
  ],
};

export const Dark: Story = {
  globals: { theme: "dark" },
  render: () => <SelectableTable />,
};

const TOP_RING = /0px 2px 0px 0px inset$/;
const SHADOW_SPLIT = /,(?![^(]*\))/;

/** Верхняя грань кольца фокуса ячейки: «цвет 0px 2px 0px 0px inset». */
function topRing(cell: Element): string | undefined {
  return getComputedStyle(cell)
    .boxShadow.split(SHADOW_SPLIT)
    .map((part) => part.trim())
    .find((part) => TOP_RING.test(part));
}

function FocusRingTable() {
  const [selectedId, setSelectedId] = useState<string | null>("v-141");
  return (
    <div className="w-[420px]">
      <DataTable
        columns={WIDE_COLUMNS}
        data={[...VERSIONS]}
        getRowId={getRowId}
        caption="Версии @acme/ui"
        selectedId={selectedId}
        onRowSelect={setSelectedId}
        actions={rowActions}
      />
    </div>
  );
}

export const FocusRingEqual: Story = {
  name: "Обводка фокуса одной толщины и цвета на закреплённых и обычных колонках",
  render: () => <FocusRingTable />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    canvas.getByRole("row", { selected: true }).focus();
    await userEvent.keyboard("{ArrowDown}");
    const row = canvas.getByRole("row", { selected: true });
    const cells = within(row).getAllByRole("cell");
    const pinnedFirst = topRing(cells[0] as Element);
    const middle = topRing(cells[2] as Element);
    const pinnedLast = topRing(cells.at(-1) as Element);
    await expect(pinnedFirst).toBeDefined();
    await expect(middle).toBe(pinnedFirst);
    await expect(pinnedLast).toBe(pinnedFirst);
  },
};
