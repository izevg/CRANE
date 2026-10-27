// i18n: не мигрирован
import type { Meta, StoryObj } from "@storybook/react-vite";
import { type ChangeEvent, useCallback, useId, useState } from "react";
import { expect, within } from "storybook/test";
import { textFormat } from "~/shared/lib/text";
import { type Action, CopyableId } from "~/shared/ui/actions";
import { columnHelper, DataTable, JsonViewer, KeyValueList } from "~/shared/ui/data-display";
import { ConfirmDialog } from "~/shared/ui/dialogs";
import { FilterSelect, SegmentedControl } from "~/shared/ui/inputs";
import { DetailPanel, PageHeader } from "~/shared/ui/layout";
import { MetricTile } from "~/shared/ui/metrics";
import {
  Badge,
  Button,
  Callout,
  CalloutDescription,
  Field,
  FieldDescription,
  FieldLabel,
  Input,
  Link,
  Tabs,
  TabsList,
  TabsTrigger,
  Textarea,
} from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import {
  AUDIT_EVENTS,
  type AuditEvent,
  DECISION_LABEL,
  type Decision,
  EVENT_HASH,
  PREV_HASH,
} from "../data/audit";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, screenParameters } from "../frame/screen";

// Макеты экранов — на русском; форматирование чисел фиксирует ru, активная локаль витрины им не нужна.
const format = textFormat("ru");

const DECISION_TONE = {
  allow: "success",
  deny: "error",
  challenge: "warning",
} as const satisfies Record<Decision, "success" | "error" | "warning">;

const COPY_JSON: readonly [Action] = [
  { label: "Копировать JSON", run: { onSelect: () => undefined } },
];
const HEADER_ACTIONS: readonly [Action, Action] = [
  { label: "Проверить целостность", run: { onSelect: () => undefined } },
  { label: "Экспорт", run: { onSelect: () => undefined }, stepUp: true },
];

const MIN_JUSTIFICATION = 20;
const noop = () => undefined;

const helper = columnHelper<AuditEvent>();
const columns = helper.columns([
  helper.accessor("time", {
    header: "Время UTC",
    meta: { priority: 1 },
    cell: (info) => <span className="font-mono text-caption tabular-nums">{info.getValue()}</span>,
  }),
  helper.accessor("actor", {
    header: "Актор",
    meta: { priority: 1 },
    cell: (info) => (
      <span className="flex flex-col leading-tight">
        <span className="font-semibold">{info.getValue().alias}</span>
        <span className="text-caption text-muted-foreground">{info.getValue().kind}</span>
      </span>
    ),
  }),
  helper.accessor("action", {
    header: "Действие",
    meta: { priority: 1 },
    cell: (info) => (
      <span className="flex flex-col leading-tight">
        <span className="font-mono text-caption">{info.getValue().code}</span>
        <span className="text-caption text-muted-foreground">{info.getValue().label}</span>
      </span>
    ),
  }),
  helper.accessor("resource", { header: "Ресурс", meta: { priority: 2 } }),
  helper.accessor("decision", {
    header: "Решение",
    meta: { priority: 1 },
    cell: (info) => (
      <StatusBadge tone={DECISION_TONE[info.getValue()]} label={DECISION_LABEL[info.getValue()]} />
    ),
  }),
  helper.accessor("rule", {
    header: "Правило",
    meta: { priority: 2 },
    cell: (info) => <span className="font-mono text-caption">{info.getValue()}</span>,
  }),
  helper.accessor("channel", { header: "Канал", meta: { priority: 3 } }),
  helper.accessor("requestId", {
    header: "request_id",
    meta: { priority: 3 },
    cell: (info) => <CopyableId value={info.getValue()} label="Скопировать request_id" />,
  }),
]);

const getRowId = (row: AuditEvent) => row.id;

const PERIODS = [
  { value: "24h", label: "Последние 24 часа" },
  { value: "7d", label: "Последние 7 дней" },
  { value: "30d", label: "Последние 30 дней" },
] as const;
const ACTIONS = [
  { value: "package.version.blocked", label: "package.version.blocked" },
  { value: "token.issued", label: "token.issued" },
  { value: "policy.cedar.updated", label: "policy.cedar.updated" },
  { value: "audit.revealed", label: "audit.revealed" },
] as const;
const DECISIONS = [
  { value: "allow", label: "Разрешено" },
  { value: "deny", label: "Отказ" },
  { value: "challenge", label: "Нужен MFA" },
] as const;
const CHANNELS = ["UI", "CLI", "API", "CI", "A2A", "SOAR"].map((value) => ({
  value,
  label: value,
}));
const ACTOR_KINDS = [
  { value: "all", label: "Все" },
  { value: "human", label: "Человек" },
  { value: "robot", label: "Робот" },
  { value: "workload", label: "Workload" },
  { value: "agent", label: "Агент" },
] as const;
type ActorKind = (typeof ACTOR_KINDS)[number]["value"];

// Спека волны 2: S9 фильтры (screens.md) — будущий feature `AuditFilters` (DateRangePicker с пресетами ещё нет).
function Filters() {
  const [period, setPeriod] = useState<string | null>("24h");
  const [action, setAction] = useState<string | null>(null);
  const [decision, setDecision] = useState<string | null>(null);
  const [channel, setChannel] = useState<string | null>(null);
  const [kind, setKind] = useState<ActorKind>("all");
  const changeKind = useCallback((next: ActorKind) => setKind(next), []);
  return (
    <div className="flex flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2">
        <FilterSelect
          label="Период"
          allLabel="Всё время"
          value={period}
          options={PERIODS}
          onValueChange={setPeriod}
        />
        <Input
          aria-label="Актор (псевдоним)"
          placeholder="Актор (псевдоним)"
          className="max-w-52"
        />
        <FilterSelect
          label="Действие"
          allLabel="Все действия"
          value={action}
          options={ACTIONS}
          onValueChange={setAction}
        />
        <FilterSelect
          label="Решение"
          allLabel="Любое решение"
          value={decision}
          options={DECISIONS}
          onValueChange={setDecision}
        />
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <SegmentedControl
          label="Тип актора"
          items={ACTOR_KINDS}
          value={kind}
          onValueChange={changeKind}
        />
        <FilterSelect
          label="Канал"
          allLabel="Все каналы"
          value={channel}
          options={CHANNELS}
          onValueChange={setChannel}
        />
        <Input
          aria-label="request_id или ресурс"
          placeholder="request_id или ресурс"
          className="max-w-56"
        />
        <Input aria-label="IP-подсеть" placeholder="IP-подсеть" className="max-w-40" />
        <div className="ml-auto flex items-center gap-3">
          <Button variant="ghost" size="sm">
            Сбросить
          </Button>
          <span className="text-body text-muted-foreground tabular-nums">
            {format.count({ count: 18_420, capped: false })} событий
          </span>
        </div>
      </div>
    </div>
  );
}

const SPARK = [310, 332, 298, 405, 388, 421, 377, 450, 436, 468, 441, 492] as const;

function Tiles({ auditor }: { auditor: boolean }) {
  return (
    <div className="grid grid-cols-1 gap-4 @xl/page:grid-cols-2 @4xl/page:grid-cols-4">
      {auditor ? null : (
        <MetricTile
          label="События за 24 ч"
          value={18_420}
          caption="пик 14:00 UTC"
          delta={{ pct: 3.1, polarity: "neutral" }}
          spark={SPARK}
        />
      )}
      {auditor ? null : (
        <MetricTile
          label="Отставание SIEM"
          value={4.2}
          unit="с"
          caption="порог 30 с"
          spark={SPARK}
        />
      )}
      <MetricTile label="Цепочка" value="Целостна" caption="проверено 14:00 UTC" />
      <MetricTile label="Последний якорь WORM" value="14:00" caption="27.10.2026 · блок 18 442" />
    </div>
  );
}

// Спека волны 2: patterns §19 «Цепочка» — будущий feature-блок `HashChain`.
function ChainBlock() {
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-h3">Цепочка</h3>
      <KeyValueList
        items={[
          {
            label: "prev_hash",
            value: <CopyableId value={PREV_HASH} label="Скопировать prev_hash" />,
          },
          { label: "hash", value: <CopyableId value={EVENT_HASH} label="Скопировать hash" /> },
        ]}
      />
      <div>
        <StatusBadge tone="success" label="Звено проверено, якорь 14:00" />
      </div>
    </section>
  );
}

function eventJson(event: AuditEvent) {
  return {
    id: event.id,
    time: event.time,
    actor: event.actor.alias,
    action: event.action.code,
    resource: event.resource,
    decision: event.decision,
    rule: event.rule,
    channel: event.channel,
    request_id: event.requestId,
  };
}

const PANEL_TABS = [
  ["summary", "Сводка"],
  ["context", "Контекст"],
  ["json", "JSON"],
] as const;
type PanelTab = (typeof PANEL_TABS)[number][0];

const RELATED = [
  ["audit.viewed", "14:07:30.801"],
  ["policy.cedar.evaluated", "14:07:31.120"],
  ["package.version.blocked", "14:07:31.482"],
] as const;

function SummaryTab({ event }: { event: AuditEvent }) {
  return (
    <div className="flex flex-col gap-6">
      <div className="grid gap-x-6 gap-y-4 sm:grid-cols-2">
        <KeyValueList
          items={[
            { label: "Время", value: event.time },
            {
              label: "Актор",
              value: (
                <span className="flex flex-wrap items-center gap-2">
                  {event.actor.alias}
                  <Badge variant="outline">Маскировано</Badge>
                </span>
              ),
            },
            { label: "on_behalf_of", value: "user-04be" },
            { label: "Действие", value: event.action.code },
            { label: "Ресурс", value: event.resource },
          ]}
        />
        <KeyValueList
          items={[
            {
              label: "Решение",
              value: (
                <StatusBadge
                  tone={DECISION_TONE[event.decision]}
                  label={DECISION_LABEL[event.decision]}
                />
              ),
            },
            { label: "Правила", value: event.rule },
            { label: "Канал", value: `${event.channel} · cctl 1.8.2` },
            { label: "IP", value: "10.12.4.0/24" },
            {
              label: "request_id",
              value: <CopyableId value={event.requestId} label="Скопировать request_id" />,
            },
            {
              label: "trace_id",
              value: (
                <CopyableId value="4bf92f3577b34da6a3ce929d0e0e4736" label="Скопировать trace_id" />
              ),
            },
          ]}
        />
      </div>
      <ChainBlock />
      <section className="flex flex-col gap-2">
        <h3 className="text-h3">События того же запроса ({RELATED.length})</h3>
        <ul className="m-0 flex list-none flex-col gap-1 p-0">
          {RELATED.map(([code, time]) => (
            <li key={code}>
              <Link href={`/audit/${code}`} className="text-brand underline">
                {code} · {time}
              </Link>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}

function EventPanel({
  event,
  auditor,
  open,
  onOpenChange,
  onReveal,
}: {
  event: AuditEvent;
  auditor: boolean;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onReveal: () => void;
}) {
  const [tab, setTab] = useState<PanelTab>("summary");
  const changeTab = useCallback((value: unknown) => setTab(value as PanelTab), []);
  const footer = auditor
    ? undefined
    : {
        primary: {
          label: "Раскрыть данные",
          run: { onSelect: onReveal },
          stepUp: true,
        } satisfies Action,
        secondary: COPY_JSON,
      };
  return (
    <DetailPanel
      open={open}
      onOpenChange={onOpenChange}
      title="Событие"
      status={
        <StatusBadge tone={DECISION_TONE[event.decision]} label={DECISION_LABEL[event.decision]} />
      }
      tabs={
        <Tabs value={tab} onValueChange={changeTab}>
          <TabsList variant="line" aria-label="Разделы события">
            {PANEL_TABS.map(([value, label]) => (
              <TabsTrigger key={value} value={value}>
                {label}
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
      }
      {...(footer ? { footer } : {})}
    >
      {tab === "summary" ? <SummaryTab event={event} /> : null}
      {tab === "context" ? (
        <KeyValueList
          items={[
            { label: "Клиент", value: "cctl 1.8.2 (linux/amd64)" },
            { label: "User-Agent", value: "cctl/1.8.2 go1.24" },
            { label: "Workspace", value: "platform" },
            { label: "Сессия", value: "ses_5be0c1d4" },
          ]}
        />
      ) : null}
      {tab === "json" ? <JsonViewer label="JSON события" value={eventJson(event)} /> : null}
    </DetailPanel>
  );
}

// Спека волны 2: patterns §20 — будущий feature `RevealDialog` (форма обоснования + step-up §21).
function RevealDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const [text, setText] = useState("");
  const counterId = useId();
  const change = useCallback(
    (event: ChangeEvent<HTMLInputElement>) => setText(event.target.value),
    [],
  );
  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title="Раскрыть данные события?"
      description="Будут показаны логин и полный IP-адрес актора."
      consequences={["Раскрытие запишется в журнал", "Данные видны только в этой панели"]}
      confirm={{
        label: "Раскрыть",
        run: { onSelect: noop },
        stepUp: true,
        ...(text.trim().length < MIN_JUSTIFICATION
          ? { disabledReason: `Обоснование: минимум ${MIN_JUSTIFICATION} символов` }
          : {}),
      }}
    >
      <Callout tone="info">
        <CalloutDescription>Потребуется подтверждение входа (MFA)</CalloutDescription>
      </Callout>
      <Field>
        <FieldLabel>Обоснование</FieldLabel>
        <Textarea value={text} onChange={change} aria-describedby={counterId} />
        <FieldDescription id={counterId}>
          {text.trim().length} из {MIN_JUSTIFICATION} символов. Раскрытие будет записано в журнал
        </FieldDescription>
      </Field>
    </ConfirmDialog>
  );
}

type ScreenProps = { auditor?: boolean; revealOpen?: boolean };

// Спека волны 2: архетипы F + C (composition §4) — будущий widget `AuditJournal`.
function S9Screen({ auditor = false, revealOpen = false }: ScreenProps) {
  const [selectedId, setSelectedId] = useState<string | null>("evt-3");
  const [panelOpen, setPanelOpen] = useState(true);
  const [reveal, setReveal] = useState(revealOpen);
  const select = useCallback((id: string) => {
    setSelectedId(id);
    setPanelOpen(true);
  }, []);
  const openReveal = useCallback(() => setReveal(true), []);
  const event = AUDIT_EVENTS.find((item) => item.id === selectedId) ?? AUDIT_EVENTS[0];
  const header = auditor
    ? { caption: "Режим аудитора: данные маскированы" }
    : {
        caption: "Хранение онлайн 12 месяцев, далее WORM-архив",
        secondary: HEADER_ACTIONS,
      };
  return (
    <ScreenFrame active="audit" breadcrumbs={{ trail: [], current: "Журнал" }}>
      <div className="flex flex-col gap-6">
        <PageHeader title="Журнал действий" {...header} />
        <Callout tone="info">
          <CalloutDescription>Каждый поиск записывается в журнал (audit.viewed)</CalloutDescription>
        </Callout>
        <Tiles auditor={auditor} />
        <Filters />
        <div className="flex items-start gap-6">
          <section aria-label="События" className="flex min-w-0 flex-1 flex-col gap-4">
            <DataTable
              columns={columns}
              data={[...AUDIT_EVENTS]}
              getRowId={getRowId}
              caption="События журнала"
              compact={panelOpen}
              selectedId={selectedId}
              onRowSelect={select}
              loadMore={{ label: "Загрузить ещё", run: { onSelect: noop } }}
            />
          </section>
          {event ? (
            <EventPanel
              event={event}
              auditor={auditor}
              open={panelOpen}
              onOpenChange={setPanelOpen}
              onReveal={openReveal}
            />
          ) : null}
        </div>
      </div>
      {auditor ? null : <RevealDialog open={reveal} onOpenChange={setReveal} />}
    </ScreenFrame>
  );
}

const meta = {
  title: "Экраны/S9 Журнал",
  component: S9Screen,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof S9Screen>;

export default meta;
type Story = StoryObj<typeof meta>;

const WIDE = { name: "Desktop 1600×900", styles: { width: "1600px", height: "900px" } };
const wideParameters = {
  ...screenParameters,
  viewport: { options: { ...screenParameters.viewport.options, wide: WIDE } },
} as const;
const wide = { viewport: { value: "wide", isRotated: false } } as const;

/** 1440: панель события открывается как Sheet. */
export const Desktop1440: Story = {
  name: "Desktop 1440 · SecOps",
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    await expect(await body.findByRole("dialog", { name: "Событие" })).toBeVisible();
  },
};

export const Desktop1600: Story = {
  name: "Desktop 1600 · панель docked",
  parameters: wideParameters,
  globals: wide,
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    await expect(
      await body.findByRole("heading", { level: 1, name: "Журнал действий" }),
    ).toBeVisible();
  },
};

export const DesktopDark: Story = {
  name: "Desktop dark · docked",
  parameters: wideParameters,
  globals: { ...wide, theme: "dark" },
};

export const Mobile390: Story = {
  name: "Mobile 390",
  globals: mobile,
};

export const AuditorMode: Story = {
  name: "Режим аудитора",
  args: { auditor: true },
  parameters: wideParameters,
  globals: wide,
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    await expect(await body.findByText("Режим аудитора: данные маскированы")).toBeVisible();
    await expect(body.queryByRole("button", { name: "Экспорт" })).toBeNull();
    await expect(body.queryByRole("button", { name: "Раскрыть данные" })).toBeNull();
  },
};

export const RevealDialogOpen: Story = {
  name: "Диалог раскрытия данных",
  args: { revealOpen: true },
  parameters: wideParameters,
  globals: wide,
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    const dialog = await body.findByRole("alertdialog", { name: "Раскрыть данные события?" });
    await expect(within(dialog).getByRole("textbox", { name: "Обоснование" })).toBeVisible();
  },
};
