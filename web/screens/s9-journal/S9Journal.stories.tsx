import { msg } from "@lingui/core/macro";
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
import { tr } from "../frame/tr";

// Макеты экранов — на русском; форматирование чисел фиксирует ru, активная локаль витрины им не нужна.
const format = textFormat("ru");

const DECISION_TONE = {
  allow: "success",
  deny: "error",
  challenge: "warning",
} as const satisfies Record<Decision, "success" | "error" | "warning">;

const COPY_JSON: readonly [Action] = [
  {
    get label() {
      return tr(msg`Копировать JSON`);
    },
    run: { onSelect: () => undefined },
  },
];
const HEADER_ACTIONS: readonly [Action, Action] = [
  {
    get label() {
      return tr(msg`Проверить целостность`);
    },
    run: { onSelect: () => undefined },
  },
  {
    get label() {
      return tr(msg`Экспорт`);
    },
    run: { onSelect: () => undefined },
    stepUp: true,
  },
];

const MIN_JUSTIFICATION = 20;
const noop = () => undefined;

const helper = columnHelper<AuditEvent>();
const columns = helper.columns([
  helper.accessor("time", {
    header: () => tr(msg`Время UTC`),
    meta: { priority: 1 },
    cell: (info) => <span className="font-mono text-caption tabular-nums">{info.getValue()}</span>,
  }),
  helper.accessor("actor", {
    header: () => tr(msg`Актор`),
    meta: { priority: 1 },
    cell: (info) => (
      <span className="flex flex-col leading-tight">
        <span className="font-semibold">{info.getValue().alias}</span>
        <span className="text-caption text-muted-foreground">{info.getValue().kind}</span>
      </span>
    ),
  }),
  helper.accessor("action", {
    header: () => tr(msg`Действие`),
    meta: { priority: 1 },
    cell: (info) => (
      <span className="flex flex-col leading-tight">
        <span className="font-mono text-caption">{info.getValue().code}</span>
        <span className="text-caption text-muted-foreground">{info.getValue().label}</span>
      </span>
    ),
  }),
  helper.accessor("resource", { header: () => tr(msg`Ресурс`), meta: { priority: 2 } }),
  helper.accessor("decision", {
    header: () => tr(msg`Решение`),
    meta: { priority: 1 },
    cell: (info) => (
      <StatusBadge tone={DECISION_TONE[info.getValue()]} label={DECISION_LABEL[info.getValue()]} />
    ),
  }),
  helper.accessor("rule", {
    header: () => tr(msg`Правило`),
    meta: { priority: 2 },
    cell: (info) => <span className="font-mono text-caption">{info.getValue()}</span>,
  }),
  helper.accessor("channel", { header: () => tr(msg`Канал`), meta: { priority: 3 } }),
  helper.accessor("requestId", {
    header: "request_id",
    meta: { priority: 3 },
    cell: (info) => <CopyableId value={info.getValue()} label={tr(msg`Скопировать request_id`)} />,
  }),
]);

const getRowId = (row: AuditEvent) => row.id;

const PERIODS = [
  {
    value: "24h",
    get label() {
      return tr(msg`Последние 24 часа`);
    },
  },
  {
    value: "7d",
    get label() {
      return tr(msg`Последние 7 дней`);
    },
  },
  {
    value: "30d",
    get label() {
      return tr(msg`Последние 30 дней`);
    },
  },
] as const;
const ACTIONS = [
  { value: "package.version.blocked", label: "package.version.blocked" },
  { value: "token.issued", label: "token.issued" },
  { value: "policy.cedar.updated", label: "policy.cedar.updated" },
  { value: "audit.revealed", label: "audit.revealed" },
] as const;
const DECISIONS = [
  {
    value: "allow",
    get label() {
      return tr(msg`Разрешено`);
    },
  },
  {
    value: "deny",
    get label() {
      return tr(msg`Отказ`);
    },
  },
  {
    value: "challenge",
    get label() {
      return tr(msg`Нужен MFA`);
    },
  },
] as const;
const CHANNELS = ["UI", "CLI", "API", "CI", "A2A", "SOAR"].map((value) => ({
  value,
  label: value,
}));
const ACTOR_KINDS = [
  {
    value: "all",
    get label() {
      return tr(msg`Все`);
    },
  },
  {
    value: "human",
    get label() {
      return tr(msg`Человек`);
    },
  },
  {
    value: "robot",
    get label() {
      return tr(msg`Робот`);
    },
  },
  { value: "workload", label: "Workload" },
  {
    value: "agent",
    get label() {
      return tr(msg`Агент`);
    },
  },
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
          label={tr(msg`Период`)}
          allLabel={tr(msg`Всё время`)}
          value={period}
          options={PERIODS}
          onValueChange={setPeriod}
        />
        <Input
          aria-label={tr(msg`Актор (псевдоним)`)}
          placeholder={tr(msg`Актор (псевдоним)`)}
          className="max-w-52"
        />
        <FilterSelect
          label={tr(msg`Действие`)}
          allLabel={tr(msg`Все действия`)}
          value={action}
          options={ACTIONS}
          onValueChange={setAction}
        />
        <FilterSelect
          label={tr(msg`Решение`)}
          allLabel={tr(msg`Любое решение`)}
          value={decision}
          options={DECISIONS}
          onValueChange={setDecision}
        />
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <SegmentedControl
          label={tr(msg`Тип актора`)}
          items={ACTOR_KINDS}
          value={kind}
          onValueChange={changeKind}
        />
        <FilterSelect
          label={tr(msg`Канал`)}
          allLabel={tr(msg`Все каналы`)}
          value={channel}
          options={CHANNELS}
          onValueChange={setChannel}
        />
        <Input
          aria-label={tr(msg`request_id или ресурс`)}
          placeholder={tr(msg`request_id или ресурс`)}
          className="max-w-56"
        />
        <Input
          aria-label={tr(msg`IP-подсеть`)}
          placeholder={tr(msg`IP-подсеть`)}
          className="max-w-40"
        />
        <div className="ml-auto flex items-center gap-3">
          <Button variant="ghost" size="sm">
            {tr(msg`Сбросить`)}
          </Button>
          <span className="text-body text-muted-foreground tabular-nums">
            {format.count({ count: 18_420, capped: false })} {tr(msg`событий`)}
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
          label={tr(msg`События за 24 ч`)}
          value={18_420}
          caption={tr(msg`пик 14:00 UTC`)}
          delta={{ pct: 3.1, polarity: "neutral" }}
          spark={SPARK}
        />
      )}
      {auditor ? null : (
        <MetricTile
          label={tr(msg`Отставание SIEM`)}
          value={4.2}
          unit={tr(msg`с`)}
          caption={tr(msg`порог 30 с`)}
          spark={SPARK}
        />
      )}
      <MetricTile
        label={tr(msg`Цепочка`)}
        value={tr(msg`Целостна`)}
        caption={tr(msg`проверено 14:00 UTC`)}
      />
      <MetricTile
        label={tr(msg`Последний якорь WORM`)}
        value="14:00"
        caption={tr(msg`27.10.2026 · блок 18 442`)}
      />
    </div>
  );
}

// Спека волны 2: patterns §19 «Цепочка» — будущий feature-блок `HashChain`.
function ChainBlock() {
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-h3">{tr(msg`Цепочка`)}</h3>
      <KeyValueList
        items={[
          {
            label: "prev_hash",
            value: <CopyableId value={PREV_HASH} label={tr(msg`Скопировать prev_hash`)} />,
          },
          {
            label: "hash",
            value: <CopyableId value={EVENT_HASH} label={tr(msg`Скопировать hash`)} />,
          },
        ]}
      />
      <div>
        <StatusBadge tone="success" label={tr(msg`Звено проверено, якорь 14:00`)} />
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
  ["summary", msg`Сводка`],
  ["context", msg`Контекст`],
  ["json", msg`JSON`],
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
            { label: tr(msg`Время`), value: event.time },
            {
              label: tr(msg`Актор`),
              value: (
                <span className="flex flex-wrap items-center gap-2">
                  {event.actor.alias}
                  <Badge variant="outline">{tr(msg`Маскировано`)}</Badge>
                </span>
              ),
            },
            { label: "on_behalf_of", value: "user-04be" },
            { label: tr(msg`Действие`), value: event.action.code },
            { label: tr(msg`Ресурс`), value: event.resource },
          ]}
        />
        <KeyValueList
          items={[
            {
              label: tr(msg`Решение`),
              value: (
                <StatusBadge
                  tone={DECISION_TONE[event.decision]}
                  label={DECISION_LABEL[event.decision]}
                />
              ),
            },
            { label: tr(msg`Правила`), value: event.rule },
            { label: tr(msg`Канал`), value: `${event.channel} · cctl 1.8.2` },
            { label: "IP", value: "10.12.4.0/24" },
            {
              label: "request_id",
              value: <CopyableId value={event.requestId} label={tr(msg`Скопировать request_id`)} />,
            },
            {
              label: "trace_id",
              value: (
                <CopyableId
                  value="4bf92f3577b34da6a3ce929d0e0e4736"
                  label={tr(msg`Скопировать trace_id`)}
                />
              ),
            },
          ]}
        />
      </div>
      <ChainBlock />
      <section className="flex flex-col gap-2">
        <h3 className="text-h3">
          {tr(msg`События того же запроса (`)}
          {RELATED.length})
        </h3>
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
          label: tr(msg`Раскрыть данные`),
          run: { onSelect: onReveal },
          stepUp: true,
        } satisfies Action,
        secondary: COPY_JSON,
      };
  return (
    <DetailPanel
      open={open}
      onOpenChange={onOpenChange}
      title={tr(msg`Событие`)}
      status={
        <StatusBadge tone={DECISION_TONE[event.decision]} label={DECISION_LABEL[event.decision]} />
      }
      tabs={
        <Tabs value={tab} onValueChange={changeTab}>
          <TabsList variant="line" aria-label={tr(msg`Разделы события`)}>
            {PANEL_TABS.map(([value, label]) => (
              <TabsTrigger key={value} value={value}>
                {tr(label)}
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
            { label: tr(msg`Клиент`), value: "cctl 1.8.2 (linux/amd64)" },
            { label: "User-Agent", value: "cctl/1.8.2 go1.24" },
            { label: "Workspace", value: "platform" },
            { label: tr(msg`Сессия`), value: "ses_5be0c1d4" },
          ]}
        />
      ) : null}
      {tab === "json" ? (
        <JsonViewer label={tr(msg`JSON события`)} value={eventJson(event)} />
      ) : null}
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
      title={tr(msg`Раскрыть данные события?`)}
      description={tr(msg`Будут показаны логин и полный IP-адрес актора.`)}
      consequences={[
        tr(msg`Раскрытие запишется в журнал`),
        tr(msg`Данные видны только в этой панели`),
      ]}
      confirm={{
        label: tr(msg`Раскрыть`),
        run: { onSelect: noop },
        stepUp: true,
        ...(text.trim().length < MIN_JUSTIFICATION
          ? { disabledReason: tr(msg`Обоснование: минимум ${MIN_JUSTIFICATION} символов`) }
          : {}),
      }}
    >
      <Callout tone="info">
        <CalloutDescription>{tr(msg`Потребуется подтверждение входа (MFA)`)}</CalloutDescription>
      </Callout>
      <Field>
        <FieldLabel>{tr(msg`Обоснование`)}</FieldLabel>
        <Textarea value={text} onChange={change} aria-describedby={counterId} />
        <FieldDescription id={counterId}>
          {text.trim().length} {tr(msg`из`)} {MIN_JUSTIFICATION}{" "}
          {tr(msg`символов. Раскрытие будет записано в журнал`)}
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
    ? { caption: tr(msg`Режим аудитора: данные маскированы`) }
    : {
        caption: tr(msg`Хранение онлайн 12 месяцев, далее WORM-архив`),
        secondary: HEADER_ACTIONS,
      };
  return (
    <ScreenFrame active="audit" breadcrumbs={{ trail: [], current: tr(msg`Журнал`) }}>
      <div className="flex flex-col gap-6">
        <PageHeader title={tr(msg`Журнал действий`)} {...header} />
        <Callout tone="info">
          <CalloutDescription>
            {tr(msg`Каждый поиск записывается в журнал (audit.viewed)`)}
          </CalloutDescription>
        </Callout>
        <Tiles auditor={auditor} />
        <Filters />
        <div className="flex items-start gap-6">
          <section aria-label={tr(msg`События`)} className="flex min-w-0 flex-1 flex-col gap-4">
            <DataTable
              columns={columns}
              data={[...AUDIT_EVENTS]}
              getRowId={getRowId}
              caption={tr(msg`События журнала`)}
              compact={panelOpen}
              selectedId={selectedId}
              onRowSelect={select}
              loadMore={{ label: tr(msg`Загрузить ещё`), run: { onSelect: noop } }}
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

export const English: Story = {
  ...Desktop1440,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async ({ canvasElement }) => {
    const body = within(canvasElement.ownerDocument.body);
    await expect(await body.findByRole("dialog", { name: "Event" })).toBeVisible();
  },
};
