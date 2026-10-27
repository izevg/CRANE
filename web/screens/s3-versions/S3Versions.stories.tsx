// i18n: не мигрирован
import type { Meta, StoryObj } from "@storybook/react-vite";
import { type ReactNode, useCallback, useId, useMemo, useState } from "react";
import { expect, within } from "storybook/test";
import { textFormat } from "~/shared/lib/text";
import { type Action, ActionButton, CopyableId } from "~/shared/ui/actions";
import {
  columnHelper,
  DataTable,
  DiffViewer,
  JsonViewer,
  KeyValueList,
  Timeline,
} from "~/shared/ui/data-display";
import { FilterSelect, SegmentedControl } from "~/shared/ui/inputs";
import { DetailPanel, PageHeader } from "~/shared/ui/layout";
import { StatChip } from "~/shared/ui/metrics";
import {
  Badge,
  Callout,
  CalloutDescription,
  CalloutTitle,
  Input,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Tabs,
  TabsList,
  TabsTrigger,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import {
  PURL,
  SHA256,
  STATE_COUNTS,
  STATE_LABEL,
  VERSION_ROWS,
  type VersionRow,
  type VersionState,
} from "../data/versions";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, screenParameters } from "../frame/screen";

// Макеты экранов — на русском; форматирование чисел фиксирует ru, активная локаль витрины им не нужна.
const format = textFormat("ru");

const TONE = {
  released: "success",
  quarantined: "warning",
  blocked: "error",
  deleted: "neutral",
} as const satisfies Record<VersionState, "success" | "warning" | "error" | "neutral">;

const SELECTED_ROW = /1\.4\.2/;

const helper = columnHelper<VersionRow>();

/** Состояние + причина в одной ячейке (patterns §9): причина блокировки не прячется в флагах. */
function StateCell({ row }: { row: VersionRow }) {
  return (
    <span className="flex flex-wrap items-center gap-2">
      <StatusBadge tone={TONE[row.state]} label={STATE_LABEL[row.state]} />
      {row.state === "blocked" ? (
        <Tooltip>
          <TooltipTrigger render={<Badge variant="destructive" tabIndex={0} />}>
            Экстренно · CVE-2026-31337
          </TooltipTrigger>
          <TooltipContent>Исправлено в 4.17.21</TooltipContent>
        </Tooltip>
      ) : null}
    </span>
  );
}

const columns = helper.columns([
  helper.accessor("version", {
    header: "Версия",
    meta: { priority: 1 },
    cell: (info) =>
      info.row.original.state === "deleted" ? (
        <s className="text-muted-foreground" title="Номер версии нельзя использовать повторно">
          {info.getValue()}
        </s>
      ) : (
        <span className="font-semibold tabular-nums">{info.getValue()}</span>
      ),
  }),
  helper.accessor("state", {
    header: "Состояние",
    meta: { priority: 1 },
    cell: (info) => <StateCell row={info.row.original} />,
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
  helper.accessor("publishedAt", {
    header: "Опубликована",
    meta: { priority: 2 },
    cell: (info) => <span className="tabular-nums">{info.getValue()}</span>,
  }),
  helper.accessor("by", {
    header: "Кем",
    meta: { priority: 2 },
    cell: (info) => {
      const by = info.getValue();
      return by.kind === "human" ? by.name : `${by.prefix} · ${by.pipeline}`;
    },
  }),
  helper.accessor("source", {
    header: "Источник",
    meta: { priority: 3 },
    cell: (info) => <span className="tabular-nums">{info.getValue()}</span>,
  }),
]);

const getRowId = (row: VersionRow) => row.id;

const SEGMENTS = [
  { value: "all", label: "Все", count: STATE_COUNTS.all },
  { value: "released", label: "Выпущенные", count: STATE_COUNTS.released },
  { value: "quarantined", label: "Карантин", count: STATE_COUNTS.quarantined },
  { value: "blocked", label: "Заблокированные", count: STATE_COUNTS.blocked },
  { value: "deleted", label: "Удалённые", count: STATE_COUNTS.deleted },
] as const;

type Segment = (typeof SEGMENTS)[number]["value"];

const rowActions = (row: VersionRow): readonly Action[] =>
  row.state === "deleted"
    ? [{ label: "Открыть", run: { href: `/packages/acme-ui/versions/${row.version}` } }]
    : [
        { label: "Открыть", run: { href: `/packages/acme-ui/versions/${row.version}` } },
        { label: "Скачать SBOM", run: { onSelect: () => undefined } },
        { label: "Удалить…", run: { onSelect: () => undefined }, destructive: true, stepUp: true },
      ];

function Section({
  title,
  aside,
  children,
}: {
  title: string;
  aside?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-h3">{title}</h3>
        {aside}
      </div>
      {children}
    </section>
  );
}

const LIFECYCLE = [
  {
    id: "published",
    time: "27.10.2026, 13:42",
    title: "Опубликована",
    caption: "crr_9f8e7d6c · ui-release",
  },
  { id: "sha", time: "27.10.2026, 13:42", title: "SHA-256 подтверждён", caption: "a88d…6973" },
  {
    id: "scan",
    time: "27.10.2026, 13:48",
    title: "Скан: найдена 1 уязвимость",
    tone: "warning",
    tag: "Карантин",
    link: { label: "CVE-2026-31337", run: { href: "/findings/CVE-2026-31337" } },
  },
  {
    id: "gates",
    time: "27.10.2026, 13:49",
    title: "Гейт безопасности: отказ",
    tone: "error",
    tag: "Не пройден",
  },
  {
    id: "request",
    time: "27.10.2026, 14:07",
    title: "Запрос на продвижение в release",
    tone: "info",
    tag: "ждёт AppSec",
  },
] as const;

const DIFF_ROWS = [
  { kind: "same", path: "name", text: '"name": "@acme/ui"' },
  { kind: "change", path: "version", before: '"1.4.1"', after: '"1.4.2"' },
  { kind: "change", path: "dependencies.lodash", before: '"^4.17.20"', after: '"^4.17.21"' },
  { kind: "same", path: "dependencies.react", text: '"react": "^19.0.0"' },
] as const;

const FILES = [
  ["package.json", 2_104],
  ["dist/index.js", 98_311],
  ["dist/index.d.ts", 14_872],
  ["dist/components/DataTable.js", 31_450],
  ["dist/components/DetailPanel.js", 18_907],
  ["README.md", 4_216],
  ["LICENSE", 11_357],
] as const;

const METADATA = {
  name: "@acme/ui",
  version: "1.4.2",
  license: "Apache-2.0",
  dist: {
    tarball: "https://crane.acme.example/npm/@acme/ui/-/ui-1.4.2.tgz",
    integrity: "sha512-9fR2kQvXr7LmT0aB5nHcYe1uWdZs8GjP",
    unpackedSize: 188_416,
  },
  dependencies: { lodash: "^4.17.21", react: "^19.0.0" },
  keywords: ["ui", "design-system", "react"],
  deprecated: null,
};

// Спека волны 2: S3 «Сводка» (screens.md) — будущий widget версии `VersionSummary`.
function SummaryTab() {
  return (
    <div className="flex flex-col gap-6">
      <Callout tone="warning">
        <CalloutTitle>Продвижение в release заблокировано: 2 причины</CalloutTitle>
        <CalloutDescription>
          Находка CVE-2026-31337 (P1) в lodash 4.17.20 · нет одобрения AppSec.
        </CalloutDescription>
      </Callout>
      <KeyValueList
        items={[
          { label: "PURL", value: <CopyableId value={PURL} full label="Скопировать PURL" /> },
          { label: "Опубликована", value: "27.10.2026, 13:42" },
          { label: "Кем", value: "crr_9f8e7d6c · ui-release" },
          { label: "Размер", value: format.bytes(188_416) },
          { label: "SHA-256", value: <CopyableId value={SHA256} label="Скопировать SHA-256" /> },
          { label: "Целевая стадия", value: "release" },
        ]}
      />
      <Section title="История">
        <Timeline label="История версии 1.4.2" items={LIFECYCLE} />
      </Section>
      <Section
        title="Изменения манифеста"
        aside={
          <ActionButton
            action={{
              label: "Сравнить с 1.4.1",
              run: { href: "/packages/acme-ui/compare/1.4.1...1.4.2" },
            }}
            variant="ghost"
          />
        }
      >
        <DiffViewer label="Изменения манифеста 1.4.1 → 1.4.2" rows={DIFF_ROWS} />
      </Section>
    </div>
  );
}

function FilesTab() {
  return (
    <Table aria-label="Файлы версии 1.4.2">
      <TableHeader>
        <TableRow>
          <TableHead>Путь</TableHead>
          <TableHead className="text-right">Размер</TableHead>
          <TableHead>SHA-256</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {FILES.map(([path, size], index) => (
          <TableRow key={path}>
            <TableCell className="font-mono text-caption">{path}</TableCell>
            <TableCell className="text-right tabular-nums">{format.bytes(size)}</TableCell>
            <TableCell>
              <CopyableId
                value={`${SHA256.slice(index * 3)}${SHA256.slice(0, index * 3)}`}
                label={`Скопировать SHA-256 файла ${path}`}
              />
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

const PANEL_TABS = [
  ["summary", "Сводка"],
  ["files", "Файлы"],
  ["history", "История"],
  ["metadata", "Метаданные"],
] as const;

type PanelTab = (typeof PANEL_TABS)[number][0];

function PanelContent({ tab }: { tab: PanelTab }) {
  if (tab === "files") {
    return <FilesTab />;
  }
  if (tab === "history") {
    return <Timeline label="История версии 1.4.2" items={LIFECYCLE} />;
  }
  if (tab === "metadata") {
    return (
      <div className="flex flex-col gap-6">
        <JsonViewer label="Метаданные версии" value={METADATA} />
        <Section title="Изменения манифеста">
          <DiffViewer label="Изменения манифеста 1.4.1 → 1.4.2" rows={DIFF_ROWS} />
        </Section>
      </div>
    );
  }
  return <SummaryTab />;
}

const noop = () => undefined;

function VersionPanel({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [tab, setTab] = useState<PanelTab>("summary");
  const changeTab = useCallback((value: unknown) => setTab(value as PanelTab), []);
  return (
    <DetailPanel
      open={open}
      onOpenChange={onOpenChange}
      title="Версия 1.4.2"
      status={<StatusBadge tone="warning" label="Карантин" />}
      menu={[{ label: "Открыть полностью", run: { href: "/packages/acme-ui/versions/1.4.2" } }]}
      tabs={
        <Tabs value={tab} onValueChange={changeTab}>
          <TabsList variant="line" aria-label="Разделы версии">
            {PANEL_TABS.map(([value, label]) => (
              <TabsTrigger key={value} value={value}>
                {label}
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
      }
      footer={{
        primary: { label: "Запросить одобрение", run: { onSelect: noop } },
        secondary: [{ label: "Yank", run: { onSelect: noop } }],
        menu: [
          { label: "Скачать SBOM и evidence", run: { onSelect: noop } },
          { label: "Удалить…", run: { onSelect: noop }, destructive: true, stepUp: true },
        ],
      }}
    >
      <PanelContent tab={tab} />
    </DetailPanel>
  );
}

function matches(row: VersionRow, segment: Segment, query: string): boolean {
  return (
    (segment === "all" || row.state === segment) &&
    row.version.toLowerCase().includes(query.trim().toLowerCase())
  );
}

type ScreenProps = { initialPanelOpen?: boolean };

// Спека волны 2: архетипы A + C (composition §4) — будущий widget `PackageVersions`.
function S3Screen({ initialPanelOpen = true }: ScreenProps) {
  const [segment, setSegment] = useState<Segment>("all");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>("v-1.4.2");
  const [panelOpen, setPanelOpen] = useState(initialPanelOpen);
  const [sort, setSort] = useState<string | null>(null);
  const preId = useId();
  const searchId = useId();
  const rows = useMemo(
    () => VERSION_ROWS.filter((row) => matches(row, segment, query)),
    [segment, query],
  );
  const select = useCallback((id: string) => {
    setSelectedId(id);
    setPanelOpen(true);
  }, []);
  const changeSegment = useCallback((next: Segment) => setSegment(next), []);
  const changeQuery = useCallback(
    (event: { target: { value: string } }) => setQuery(event.target.value),
    [],
  );
  return (
    <ScreenFrame
      active="packages"
      breadcrumbs={{
        trail: [
          { label: "Мои пакеты", href: "/packages" },
          { label: "npm-hosted", href: "/repositories/npm-hosted" },
        ],
        current: "@acme/ui",
      }}
    >
      <div className="flex flex-col gap-6">
        <PageHeader
          title="@acme/ui"
          status={<StatusBadge tone="success" label="Активен" />}
          chips={
            <>
              <Badge variant="outline">npm</Badge>
              <Badge variant="outline">npm-hosted · hosted</Badge>
            </>
          }
          caption="Библиотека интерфейсных компонентов команды web. Владелец: Иванов И."
          stats={
            <>
              <StatChip label="Версий" value={STATE_COUNTS.all} />
              <StatChip label="Последняя" value="1.4.2" />
              <StatChip label="Загрузок за 30 дн" value="12 340" />
            </>
          }
          tabs={
            <Tabs value="versions">
              <TabsList variant="line" aria-label="Разделы пакета">
                <TabsTrigger value="overview">Обзор</TabsTrigger>
                <TabsTrigger value="versions">Версии</TabsTrigger>
                <TabsTrigger value="risks">Риски</TabsTrigger>
                <TabsTrigger value="members">Участники</TabsTrigger>
              </TabsList>
            </Tabs>
          }
        />
        <div className="flex items-start gap-6">
          <section aria-label="Версии" className="flex min-w-0 flex-1 flex-col gap-4">
            <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
              <SegmentedControl
                label="Состояние версий"
                items={SEGMENTS}
                value={segment}
                onValueChange={changeSegment}
              />
              <div className="flex items-center gap-2">
                <Switch id={preId} size="sm" />
                <label htmlFor={preId} className="text-body">
                  Pre-release
                </label>
              </div>
              <FilterSelect
                label="Сортировка"
                allLabel="Сначала новые"
                value={sort}
                options={[
                  { value: "published", label: "Сначала старые" },
                  { value: "downloads", label: "По загрузкам" },
                ]}
                onValueChange={setSort}
              />
              <Input
                id={searchId}
                type="search"
                aria-label="Поиск по версии или тегу"
                placeholder="Поиск по версии или тегу"
                className="max-w-64"
                value={query}
                onChange={changeQuery}
              />
            </div>
            <DataTable
              columns={columns}
              data={[...rows]}
              getRowId={getRowId}
              caption="Версии @acme/ui"
              compact={panelOpen}
              selectedId={selectedId}
              onRowSelect={select}
              actions={rowActions}
              loadMore={{ label: "Загрузить ещё", run: { onSelect: noop } }}
              empty={
                <p className="p-6 text-center text-muted-foreground">
                  Версий с такими условиями нет
                </p>
              }
            />
          </section>
          <VersionPanel open={panelOpen} onOpenChange={setPanelOpen} />
        </div>
      </div>
    </ScreenFrame>
  );
}

const meta = {
  title: "Экраны/S3 Версии",
  component: S3Screen,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof S3Screen>;

export default meta;
type Story = StoryObj<typeof meta>;

const WIDE = { name: "Desktop 1600×900", styles: { width: "1600px", height: "900px" } };
const wideParameters = {
  ...screenParameters,
  viewport: { options: { ...screenParameters.viewport.options, wide: WIDE } },
} as const;

async function expectScreen(canvasElement: HTMLElement) {
  const body = within(canvasElement.ownerDocument.body);
  await expect(await body.findByRole("heading", { level: 1, name: "@acme/ui" })).toBeVisible();
  await expect(body.getByRole("row", { name: SELECTED_ROW, selected: true })).toBeVisible();
}

/** 1440: панель открывается поверх таблицы как Sheet (правило кита: docked только от 1536). */
export const Desktop1440Sheet: Story = {
  name: "Desktop 1440 · Sheet",
  play: async ({ canvasElement }) => {
    // Модальный Sheet делает страницу за ним инертной: проверяем саму панель
    const body = within(canvasElement.ownerDocument.body);
    await expect(await body.findByRole("dialog", { name: "Версия 1.4.2" })).toBeVisible();
  },
};

/** 1600: панель docked справа от таблицы, колонки приоритета 3 скрыты. */
export const Desktop1600Docked: Story = {
  name: "Desktop 1600 · панель docked",
  parameters: wideParameters,
  globals: { viewport: { value: "wide", isRotated: false } },
  play: async ({ canvasElement }) => {
    await expectScreen(canvasElement);
  },
};

export const DesktopDark: Story = {
  name: "Desktop dark · docked",
  parameters: wideParameters,
  globals: { viewport: { value: "wide", isRotated: false }, theme: "dark" },
};

export const Desktop1440Dark: Story = {
  name: "Desktop 1440 dark · Sheet",
  globals: { ...desktop, theme: "dark" },
};

export const Mobile390: Story = {
  name: "Mobile 390 · список",
  args: { initialPanelOpen: false },
  globals: mobile,
};

export const Mobile390Panel: Story = {
  name: "Mobile 390 · панель",
  globals: mobile,
};
