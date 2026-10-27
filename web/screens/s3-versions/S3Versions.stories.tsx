import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { type ReactNode, useCallback, useId, useMemo, useState } from "react";
import { expect, within } from "storybook/test";
import { VersionFlags, VersionNumber, VersionStatus } from "~/entities/version";
import { i18n } from "~/shared/lib/i18n";
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
} from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { PURL, SHA256, STATE_COUNTS, VERSION_ROWS, type VersionRow } from "../data/versions";
import { fitsViewport } from "../frame/fit";
import { activeFormat } from "../frame/format";
import { Moment } from "../frame/Moment";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, mobile320, screenParameters } from "../frame/screen";

const SELECTED_ROW = /1\.4\.2/;
/** Момент публикации выбранной версии: тот же, что первое событие ленты, таблицы и сведений. */
const SELECTED_AT = "2026-10-27T13:42:00Z";

const helper = columnHelper<VersionRow>();

const columns = helper.columns([
  helper.accessor("version", {
    header: () => i18n._(msg`Версия`),
    meta: { priority: 1 },
    cell: (info) => <VersionNumber name={info.getValue()} state={info.row.original.state} />,
  }),
  helper.accessor("state", {
    header: () => i18n._(msg`Состояние`),
    meta: { priority: 1 },
    cell: (info) => <VersionStatus version={info.row.original} />,
  }),
  helper.accessor("flags", {
    header: () => i18n._(msg`Флаги`),
    meta: { priority: 2 },
    cell: (info) => <VersionFlags flags={info.getValue()} />,
  }),
  helper.accessor("publishedAt", {
    header: () => i18n._(msg`Опубликована`),
    meta: { priority: 2 },
    cell: (info) => <Moment iso={info.getValue()} />,
  }),
  helper.accessor("by", {
    header: () => i18n._(msg`Кем`),
    meta: { priority: 2 },
    cell: (info) => {
      const by = info.getValue();
      return by.kind === "human" ? by.name : `${by.prefix} · ${by.pipeline}`;
    },
  }),
  helper.accessor("source", {
    header: () => i18n._(msg`Источник`),
    meta: { priority: 3 },
    cell: (info) => <Moment iso={info.getValue()} />,
  }),
]);

const getRowId = (row: VersionRow) => row.id;

function segments() {
  return [
    {
      value: "all",
      label: i18n._(msg`Все`),
      count: STATE_COUNTS.all,
    },
    {
      value: "released",
      label: i18n._(msg`Выпущенные`),
      count: STATE_COUNTS.released,
    },
    {
      value: "quarantined",
      label: i18n._(msg`Карантин`),
      count: STATE_COUNTS.quarantined,
    },
    {
      value: "blocked",
      label: i18n._(msg`Заблокированные`),
      count: STATE_COUNTS.blocked,
    },
    {
      value: "deleted",
      label: i18n._(msg`Удалённые`),
      count: STATE_COUNTS.deleted,
    },
  ] as const;
}

type Segment = ReturnType<typeof segments>[number]["value"];

const rowActions = (row: VersionRow): readonly Action[] =>
  row.state === "deleted"
    ? [{ label: i18n._(msg`Открыть`), run: { href: `/packages/acme-ui/versions/${row.version}` } }]
    : [
        { label: i18n._(msg`Открыть`), run: { href: `/packages/acme-ui/versions/${row.version}` } },
        { label: i18n._(msg`Скачать SBOM`), run: { onSelect: () => undefined } },
        {
          label: i18n._(msg`Удалить…`),
          run: { onSelect: () => undefined },
          destructive: true,
          stepUp: true,
        },
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

const lifecycle = () =>
  [
    {
      id: "published",
      time: SELECTED_AT,
      title: i18n._(msg`Опубликована`),
      actor: "crr_9f8e7d6c · ui-release",
      source: "pipeline #4521",
    },
    {
      id: "sha",
      time: "2026-10-27T13:42:10Z",
      title: i18n._(msg`SHA-256 подтверждён`),
      detail: <CopyableId value={SHA256} label={i18n._(msg`Скопировать SHA-256`)} />,
    },
    {
      id: "scan",
      time: "2026-10-27T13:48:00Z",
      title: i18n._(msg`Скан: найдена 1 уязвимость`),
      tone: "warning",
      status: {
        tone: "warning",
        label: i18n._(msg`Карантин`),
      },
      source: "Trivy",
      link: { label: "CVE-2026-31337", run: { href: "/findings/CVE-2026-31337" } },
    },
    {
      id: "gates",
      time: "2026-10-27T13:49:00Z",
      title: i18n._(msg`Гейт безопасности: отказ`),
      tone: "error",
      status: {
        tone: "error",
        label: i18n._(msg`Не пройден`),
      },
    },
    {
      id: "request",
      time: "2026-10-27T14:07:00Z",
      title: i18n._(msg`Запрос на продвижение в release`),
      tone: "info",
      status: {
        tone: "info",
        label: i18n._(msg`Ждёт AppSec`),
      },
      actor: i18n._(msg`Иванов И.`),
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
  const { t } = useLingui();
  return (
    <div className="flex flex-col gap-6">
      <Callout tone="warning">
        <CalloutTitle>{t`Продвижение в release заблокировано: 2 причины`}</CalloutTitle>
        <CalloutDescription>
          {t`Находка CVE-2026-31337 (P1) в lodash 4.17.20 · нет одобрения AppSec.`}
        </CalloutDescription>
      </Callout>
      <KeyValueList
        items={[
          {
            label: "PURL",
            value: <CopyableId marked={false} value={PURL} full label={t`Скопировать PURL`} />,
          },
          { label: t`Опубликована`, value: <Moment iso={SELECTED_AT} /> },
          { label: t`Кем`, value: "crr_9f8e7d6c · ui-release" },
          { label: t`Размер`, value: activeFormat().bytes(188_416) },
          {
            label: "SHA-256",
            value: <CopyableId value={SHA256} label={t`Скопировать SHA-256`} />,
          },
          { label: t`Целевая стадия`, value: "release" },
        ]}
      />
      <Section title={t`История`}>
        <Timeline label={t`История версии 1.4.2`} items={lifecycle()} />
      </Section>
      <Section
        title={t`Изменения манифеста`}
        aside={
          <ActionButton
            action={{
              label: t`Сравнить с 1.4.1`,
              run: { href: "/packages/acme-ui/compare/1.4.1...1.4.2" },
            }}
            variant="ghost"
          />
        }
      >
        <DiffViewer label={t`Изменения манифеста 1.4.1 → 1.4.2`} rows={DIFF_ROWS} />
      </Section>
    </div>
  );
}

function FilesTab() {
  const { t } = useLingui();
  return (
    <Table aria-label={t`Файлы версии 1.4.2`}>
      <TableHeader>
        <TableRow>
          <TableHead>{t`Путь`}</TableHead>
          <TableHead className="text-right">{t`Размер`}</TableHead>
          <TableHead>SHA-256</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {FILES.map(([path, size], index) => (
          <TableRow key={path}>
            <TableCell className="font-mono text-caption">{path}</TableCell>
            <TableCell className="text-right tabular-nums">{activeFormat().bytes(size)}</TableCell>
            <TableCell>
              <CopyableId
                value={`${SHA256.slice(index * 3)}${SHA256.slice(0, index * 3)}`}
                label={t`Скопировать SHA-256 файла ${path}`}
              />
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

function panelTabs() {
  return [
    ["summary", i18n._(msg`Сводка`)],
    ["files", i18n._(msg`Файлы`)],
    ["history", i18n._(msg`История`)],
    ["metadata", i18n._(msg`Метаданные`)],
  ] as const;
}

type PanelTab = ReturnType<typeof panelTabs>[number][0];

function PanelContent({ tab }: { tab: PanelTab }) {
  const { t } = useLingui();
  if (tab === "files") {
    return <FilesTab />;
  }
  if (tab === "history") {
    return <Timeline label={t`История версии 1.4.2`} items={lifecycle()} />;
  }
  if (tab === "metadata") {
    return (
      <div className="flex flex-col gap-6">
        <JsonViewer label={t`Метаданные версии`} value={METADATA} />
        <Section title={t`Изменения манифеста`}>
          <DiffViewer label={t`Изменения манифеста 1.4.1 → 1.4.2`} rows={DIFF_ROWS} />
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
  const { t } = useLingui();
  const [tab, setTab] = useState<PanelTab>("summary");
  const changeTab = useCallback((value: unknown) => setTab(value as PanelTab), []);
  return (
    <DetailPanel
      open={open}
      onOpenChange={onOpenChange}
      title={t`Версия 1.4.2`}
      status={<VersionStatus version={{ state: "quarantined", block_reason: null }} />}
      menu={[
        {
          label: t`Открыть полностью`,
          run: { href: "/packages/acme-ui/versions/1.4.2" },
        },
      ]}
      tabs={
        <Tabs value={tab} onValueChange={changeTab}>
          <TabsList variant="line" aria-label={t`Разделы версии`}>
            {panelTabs().map(([value, label]) => (
              <TabsTrigger key={value} value={value}>
                {label}
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
      }
      footer={{
        primary: { label: t`Запросить одобрение`, run: { onSelect: noop } },
        secondary: [{ label: "Yank", run: { onSelect: noop } }],
        menu: [
          { label: t`Скачать SBOM и evidence`, run: { onSelect: noop } },
          {
            label: t`Удалить…`,
            run: { onSelect: noop },
            destructive: true,
            stepUp: true,
          },
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
  const { t } = useLingui();
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
          { label: t`Мои пакеты`, href: "/packages" },
          { label: "npm-hosted", href: "/repositories/npm-hosted" },
        ],
        current: "@acme/ui",
      }}
    >
      <div className="flex flex-col gap-6">
        <PageHeader
          title="@acme/ui"
          status={<StatusBadge tone="success" label={t`Активен`} />}
          chips={
            <>
              <Badge variant="outline">npm</Badge>
              <Badge variant="outline">npm-hosted · hosted</Badge>
            </>
          }
          caption={i18n._(
            msg`Библиотека интерфейсных компонентов команды web. Владелец: Иванов И.`,
          )}
          stats={
            <>
              <StatChip label={t`Версий`} value={STATE_COUNTS.all} />
              <StatChip label={t`Последняя`} value={<span className="font-mono">1.4.2</span>} />
              <StatChip label={t`Загрузок за 30 дн`} value="12 340" />
            </>
          }
          tabs={
            <Tabs value="versions">
              <TabsList variant="line" aria-label={t`Разделы пакета`}>
                <TabsTrigger value="overview">{t`Обзор`}</TabsTrigger>
                <TabsTrigger value="versions">{t`Версии`}</TabsTrigger>
                <TabsTrigger value="risks">{t`Риски`}</TabsTrigger>
                <TabsTrigger value="members">{t`Участники`}</TabsTrigger>
              </TabsList>
            </Tabs>
          }
        />
        <div className="flex items-start gap-6">
          <section aria-label={t`Версии`} className="flex min-w-0 flex-1 flex-col gap-4">
            <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
              <SegmentedControl
                label={t`Состояние версий`}
                items={segments()}
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
                label={t`Сортировка`}
                allLabel={t`Сначала новые`}
                value={sort}
                options={[
                  { value: "published", label: t`Сначала старые` },
                  { value: "downloads", label: t`По загрузкам` },
                ]}
                onValueChange={setSort}
              />
              <Input
                id={searchId}
                type="search"
                aria-label={t`Поиск по версии или тегу`}
                placeholder={t`Поиск по версии или тегу`}
                className="max-w-64"
                value={query}
                onChange={changeQuery}
              />
            </div>
            <DataTable
              columns={columns}
              data={[...rows]}
              getRowId={getRowId}
              caption={t`Версии @acme/ui`}
              compact={panelOpen}
              selectedId={selectedId}
              onRowSelect={select}
              actions={rowActions}
              loadMore={{ label: t`Загрузить ещё`, run: { onSelect: noop } }}
              empty={
                <p className="p-6 text-center text-muted-foreground">
                  {t`Версий с такими условиями нет`}
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
    // Версия — машинное значение: моно JetBrains, а не шрифт текста
    const table = canvasElement.querySelector("table");
    const version = within(table as HTMLElement).getAllByText("1.4.1")[0];
    await expect(getComputedStyle(version as HTMLElement).fontFamily).toContain("JetBrains Mono");
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

export const English: Story = {
  ...Desktop1440Sheet,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    await expect(await body.findByRole("dialog", { name: "Version 1.4.2" })).toBeVisible();
    // Подписи состояний — словарь интерфейса: в EN таблица и панель одного языка
    const table = canvasElement.querySelector("table");
    await expect(table?.textContent).toContain("Quarantine");
    await expect(table?.textContent).not.toContain("Карантин");
    await expect(table?.textContent).not.toContain("Выпущена");
  },
};

/** Топбар не шире окна на телефоне (WCAG 1.4.10): ширины 390 и 320, обе локали. */
export const Fit390: Story = {
  ...Mobile390,
  name: "Телефон 390: топбар помещается",
  play: fitsViewport,
};

export const Fit320: Story = {
  ...Mobile390,
  name: "Телефон 320: топбар помещается",
  globals: mobile320,
  play: fitsViewport,
};

export const Fit390English: Story = {
  ...Mobile390,
  name: "Телефон 390 EN: топбар помещается",
  globals: { ...mobile, locale: "en" },
  play: fitsViewport,
};

export const Fit320English: Story = {
  ...Mobile390,
  name: "Телефон 320 EN: топбар помещается",
  globals: { ...mobile320, locale: "en" },
  play: fitsViewport,
};
