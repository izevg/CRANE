import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { useCallback, useState } from "react";
import { expect, within } from "storybook/test";
import { i18n } from "~/shared/lib/i18n";
import { useSessionDraft } from "~/shared/lib/state";
import { type Action, CopyableId } from "~/shared/ui/actions";
import { columnHelper, DataTable, JsonViewer, KeyValueList } from "~/shared/ui/data-display";
import { ConfirmDialog } from "~/shared/ui/dialogs";
import {
  FilterSelect,
  JUSTIFICATION_MIN,
  JustificationField,
  justificationStatus,
  SegmentedControl,
} from "~/shared/ui/inputs";
import { DetailPanel, PageHeader } from "~/shared/ui/layout";
import { MetricTile } from "~/shared/ui/metrics";
import {
  Badge,
  Button,
  Callout,
  CalloutDescription,
  Input,
  Link,
  Tabs,
  TabsList,
  TabsTrigger,
} from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { AUDIT_EVENTS, type AuditEvent, EVENT_HASH, PREV_HASH } from "../data/audit";
import { actorKindLabel, decisionBadge } from "../data/states";
import { activeFormat } from "../frame/format";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, screenParameters } from "../frame/screen";

function copyJson(): readonly [Action] {
  return [
    {
      label: i18n._(msg`Копировать JSON`),
      run: { onSelect: () => undefined },
    },
  ];
}
function headerActions(): readonly [Action, Action] {
  return [
    {
      label: i18n._(msg`Проверить целостность`),
      run: { onSelect: () => undefined },
    },
    {
      label: i18n._(msg`Экспорт`),
      run: { onSelect: () => undefined },
      stepUp: true,
    },
  ];
}

const MIN_JUSTIFICATION = JUSTIFICATION_MIN;
const noop = () => undefined;

const helper = columnHelper<AuditEvent>();
const columns = helper.columns([
  helper.accessor("time", {
    header: () => i18n._(msg`Время UTC`),
    meta: { priority: 1 },
    cell: (info) => <span className="font-mono text-caption tabular-nums">{info.getValue()}</span>,
  }),
  helper.accessor("actor", {
    header: () => i18n._(msg`Актор`),
    meta: { priority: 1 },
    cell: (info) => (
      <span className="flex flex-col leading-tight">
        <span className="font-semibold">{info.getValue().alias}</span>
        <span className="text-caption text-muted-foreground">
          {actorKindLabel(info.getValue().kind)}
        </span>
      </span>
    ),
  }),
  helper.accessor("action", {
    header: () => i18n._(msg`Действие`),
    meta: { priority: 1 },
    cell: (info) => (
      <span className="flex flex-col leading-tight">
        <span className="font-mono text-caption">{info.getValue().code}</span>
        <span className="text-caption text-muted-foreground">{info.getValue().label}</span>
      </span>
    ),
  }),
  helper.accessor("resource", { header: () => i18n._(msg`Ресурс`), meta: { priority: 2 } }),
  helper.accessor("decision", {
    header: () => i18n._(msg`Решение`),
    meta: { priority: 1 },
    cell: (info) => <StatusBadge {...decisionBadge(info.getValue())} />,
  }),
  helper.accessor("rule", {
    header: () => i18n._(msg`Правило`),
    meta: { priority: 2 },
    cell: (info) => <span className="font-mono text-caption">{info.getValue()}</span>,
  }),
  helper.accessor("channel", { header: () => i18n._(msg`Канал`), meta: { priority: 3 } }),
  helper.accessor("requestId", {
    header: "request_id",
    meta: { priority: 3 },
    cell: (info) => (
      <CopyableId value={info.getValue()} label={i18n._(msg`Скопировать request_id`)} />
    ),
  }),
]);

const getRowId = (row: AuditEvent) => row.id;

function periods() {
  return [
    {
      value: "24h",
      label: i18n._(msg`Последние 24 часа`),
    },
    {
      value: "7d",
      label: i18n._(msg`Последние 7 дней`),
    },
    {
      value: "30d",
      label: i18n._(msg`Последние 30 дней`),
    },
  ] as const;
}
const ACTIONS = [
  { value: "package.version.blocked", label: "package.version.blocked" },
  { value: "token.issued", label: "token.issued" },
  { value: "policy.cedar.updated", label: "policy.cedar.updated" },
  { value: "audit.revealed", label: "audit.revealed" },
] as const;
function decisions() {
  return [
    {
      value: "allow",
      label: i18n._(msg`Разрешено`),
    },
    {
      value: "deny",
      label: i18n._(msg`Отказ`),
    },
    {
      value: "challenge",
      label: i18n._(msg`Нужен MFA`),
    },
  ] as const;
}
const CHANNELS = ["UI", "CLI", "API", "CI", "A2A", "SOAR"].map((value) => ({
  value,
  label: value,
}));
function actorKinds() {
  return [
    {
      value: "all",
      label: i18n._(msg`Все`),
    },
    {
      value: "human",
      label: i18n._(msg`Человек`),
    },
    {
      value: "robot",
      label: i18n._(msg`Робот`),
    },
    { value: "workload", label: "Workload" },
    {
      value: "agent",
      label: i18n._(msg`Агент`),
    },
  ] as const;
}
type ActorKind = ReturnType<typeof actorKinds>[number]["value"];

// Спека волны 2: S9 фильтры (screens.md) — будущий feature `AuditFilters` (DateRangePicker с пресетами ещё нет).
function Filters() {
  const { t } = useLingui();
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
          label={t`Период`}
          allLabel={t`Всё время`}
          value={period}
          options={periods()}
          onValueChange={setPeriod}
        />
        <Input
          aria-label={t`Актор (псевдоним)`}
          placeholder={t`Актор (псевдоним)`}
          className="max-w-52"
        />
        <FilterSelect
          label={t`Действие`}
          allLabel={t`Все действия`}
          value={action}
          options={ACTIONS}
          onValueChange={setAction}
        />
        <FilterSelect
          label={t`Решение`}
          allLabel={t`Любое решение`}
          value={decision}
          options={decisions()}
          onValueChange={setDecision}
        />
      </div>
      <div className="flex flex-wrap items-center gap-2">
        <SegmentedControl
          label={t`Тип актора`}
          items={actorKinds()}
          value={kind}
          onValueChange={changeKind}
        />
        <FilterSelect
          label={t`Канал`}
          allLabel={t`Все каналы`}
          value={channel}
          options={CHANNELS}
          onValueChange={setChannel}
        />
        <Input
          aria-label={t`request_id или ресурс`}
          placeholder={t`request_id или ресурс`}
          className="max-w-56"
        />
        <Input aria-label={t`IP-подсеть`} placeholder={t`IP-подсеть`} className="max-w-40" />
        <div className="ml-auto flex items-center gap-3">
          <Button variant="ghost" size="sm">
            {t`Сбросить`}
          </Button>
          <span className="text-body text-muted-foreground tabular-nums">
            {activeFormat().count({ count: 18_420, capped: false })} {t`событий`}
          </span>
        </div>
      </div>
    </div>
  );
}

const SPARK = [310, 332, 298, 405, 388, 421, 377, 450, 436, 468, 441, 492] as const;

function Tiles({ auditor }: { auditor: boolean }) {
  const { t } = useLingui();
  return (
    <div className="grid grid-cols-1 gap-4 @xl/page:grid-cols-2 @4xl/page:grid-cols-4">
      {auditor ? null : (
        <MetricTile
          label={t`События за 24 ч`}
          value={18_420}
          caption={t`пик 14:00 UTC`}
          delta={{ pct: 3.1, polarity: "neutral" }}
          spark={SPARK}
        />
      )}
      {auditor ? null : (
        <MetricTile
          label={t`Отставание SIEM`}
          value={4.2}
          unit={t`с`}
          caption={t`порог 30 с`}
          spark={SPARK}
        />
      )}
      <MetricTile label={t`Цепочка`} value={t`Целостна`} caption={t`проверено 14:00 UTC`} />
      <MetricTile
        label={t`Последний якорь WORM`}
        value="14:00"
        caption={t`27.10.2026 · блок 18 442`}
      />
    </div>
  );
}

// Спека волны 2: patterns §19 «Цепочка» — будущий feature-блок `HashChain`.
function ChainBlock() {
  const { t } = useLingui();
  return (
    <section className="flex flex-col gap-2">
      <h3 className="text-h3">{t`Цепочка`}</h3>
      <KeyValueList
        items={[
          {
            label: "prev_hash",
            value: <CopyableId value={PREV_HASH} label={t`Скопировать prev_hash`} />,
          },
          {
            label: "hash",
            value: <CopyableId value={EVENT_HASH} label={t`Скопировать hash`} />,
          },
        ]}
      />
      <div>
        <StatusBadge tone="success" label={t`Звено проверено, якорь 14:00`} />
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

function panelTabs() {
  return [
    ["summary", i18n._(msg`Сводка`)],
    ["context", i18n._(msg`Контекст`)],
    ["json", i18n._(msg`JSON`)],
  ] as const;
}
type PanelTab = ReturnType<typeof panelTabs>[number][0];

const RELATED = [
  ["audit.viewed", "14:07:30.801"],
  ["policy.cedar.evaluated", "14:07:31.120"],
  ["package.version.blocked", "14:07:31.482"],
] as const;

function SummaryTab({ event }: { event: AuditEvent }) {
  const { t } = useLingui();
  return (
    <div className="flex flex-col gap-6">
      <div className="grid gap-x-6 gap-y-4 sm:grid-cols-2">
        <KeyValueList
          items={[
            { label: t`Время`, value: event.time },
            {
              label: t`Актор`,
              value: (
                <span className="flex flex-wrap items-center gap-2">
                  {event.actor.alias}
                  <Badge variant="outline">{t`Маскировано`}</Badge>
                </span>
              ),
            },
            { label: "on_behalf_of", value: "user-04be" },
            { label: t`Действие`, value: event.action.code },
            { label: t`Ресурс`, value: event.resource },
          ]}
        />
        <KeyValueList
          items={[
            {
              label: t`Решение`,
              value: <StatusBadge {...decisionBadge(event.decision)} />,
            },
            { label: t`Правила`, value: event.rule },
            { label: t`Канал`, value: `${event.channel} · cctl 1.8.2` },
            { label: "IP", value: "10.12.4.0/24" },
            {
              label: "request_id",
              value: <CopyableId value={event.requestId} label={t`Скопировать request_id`} />,
            },
            {
              label: "trace_id",
              value: (
                <CopyableId
                  value="4bf92f3577b34da6a3ce929d0e0e4736"
                  label={t`Скопировать trace_id`}
                />
              ),
            },
          ]}
        />
      </div>
      <ChainBlock />
      <section className="flex flex-col gap-2">
        <h3 className="text-h3">
          {t`События того же запроса (`}
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
  const { t } = useLingui();
  const [tab, setTab] = useState<PanelTab>("summary");
  const changeTab = useCallback((value: unknown) => setTab(value as PanelTab), []);
  const footer = auditor
    ? undefined
    : {
        primary: {
          label: t`Раскрыть данные`,
          run: { onSelect: onReveal },
          stepUp: true,
        } satisfies Action,
        secondary: copyJson(),
      };
  return (
    <DetailPanel
      open={open}
      onOpenChange={onOpenChange}
      title={t`Событие`}
      status={<StatusBadge {...decisionBadge(event.decision)} />}
      tabs={
        <Tabs value={tab} onValueChange={changeTab}>
          <TabsList variant="line" aria-label={t`Разделы события`}>
            {panelTabs().map(([value, label]) => (
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
            { label: t`Клиент`, value: "cctl 1.8.2 (linux/amd64)" },
            { label: "User-Agent", value: "cctl/1.8.2 go1.24" },
            { label: "Workspace", value: "platform" },
            { label: t`Сессия`, value: "ses_5be0c1d4" },
          ]}
        />
      ) : null}
      {tab === "json" ? <JsonViewer label={t`JSON события`} value={eventJson(event)} /> : null}
    </DetailPanel>
  );
}

const isText = (value: unknown): value is string => typeof value === "string";

/** Раскрытие данных (patterns §20): ConfirmDialog с обоснованием; введённое переживает редирект step-up. */
function RevealDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useLingui();
  // Текст обоснования живёт только в черновике: он же переживает редирект step-up
  const [draft, saveDraft, clearDraft] = useSessionDraft("reveal-event", isText);
  const text = draft ?? "";
  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t`Раскрыть данные события?`}
      description={t`Будут показаны логин и полный IP-адрес актора.`}
      consequences={[t`Раскрытие запишется в журнал`, t`Данные видны только в этой панели`]}
      confirm={{
        label: t`Раскрыть`,
        run: { onSelect: clearDraft },
        stepUp: true,
        ...(justificationStatus(text) === "short"
          ? { disabledReason: t`Обоснование: минимум ${MIN_JUSTIFICATION} символов` }
          : {}),
      }}
    >
      <Callout tone="info">
        <CalloutDescription>{t`Потребуется подтверждение входа (MFA)`}</CalloutDescription>
      </Callout>
      <JustificationField label={t`Обоснование`} value={text} onChange={saveDraft} />
    </ConfirmDialog>
  );
}

type ScreenProps = { auditor?: boolean; revealOpen?: boolean };

// Спека волны 2: архетипы F + C (composition §4) — будущий widget `AuditJournal`.
function S9Screen({ auditor = false, revealOpen = false }: ScreenProps) {
  const { t } = useLingui();
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
    ? { caption: t`Режим аудитора: данные маскированы` }
    : {
        caption: t`Хранение онлайн 12 месяцев, далее WORM-архив`,
        secondary: headerActions(),
      };
  return (
    <ScreenFrame active="audit" breadcrumbs={{ trail: [], current: t`Журнал` }}>
      <div className="flex flex-col gap-6">
        <PageHeader title={t`Журнал действий`} {...header} />
        <Callout tone="info">
          <CalloutDescription>
            {t`Каждый поиск записывается в журнал (audit.viewed)`}
          </CalloutDescription>
        </Callout>
        <Tiles auditor={auditor} />
        <Filters />
        <div className="flex items-start gap-6">
          <section aria-label={t`События`} className="flex min-w-0 flex-1 flex-col gap-4">
            <DataTable
              columns={columns}
              data={[...AUDIT_EVENTS]}
              getRowId={getRowId}
              caption={t`События журнала`}
              compact={panelOpen}
              selectedId={selectedId}
              onRowSelect={select}
              loadMore={{ label: t`Загрузить ещё`, run: { onSelect: noop } }}
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
    // Тип актора в таблице — подпись интерфейса, а не контент: по-английски
    await expect(body.getAllByText("Human").length).toBeGreaterThan(0);
  },
};
