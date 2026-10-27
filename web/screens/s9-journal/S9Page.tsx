import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { type ReactNode, useCallback, useState } from "react";
import { i18n } from "~/shared/lib/i18n";
import { type Action, CopyableId } from "~/shared/ui/actions";
import { columnHelper, DataTable } from "~/shared/ui/data-display";
import { PageHeader } from "~/shared/ui/layout";
import { MetricTile } from "~/shared/ui/metrics";
import { Callout, CalloutDescription } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { AUDIT_EVENTS, type AuditEvent, SPARK } from "../data/audit";
import { actorKindLabel, decisionBadge } from "../data/states";
import { ScreenFrame } from "../frame/ScreenFrame";
import { Filters } from "./S9Filters";
import { EventPanel } from "./S9Panel";

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

export type RevealSlot = (state: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) => ReactNode;

type PageProps = { auditor?: boolean; revealOpen?: boolean; renderReveal: RevealSlot };

// Спека волны 2: архетипы F + C (composition §4) — будущий widget `AuditJournal`.
export function S9Page({ auditor = false, revealOpen = false, renderReveal }: PageProps) {
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
      {auditor ? null : renderReveal({ open: reveal, onOpenChange: setReveal })}
    </ScreenFrame>
  );
}
