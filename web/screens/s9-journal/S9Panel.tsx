import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useCallback, useState } from "react";
import { i18n } from "~/shared/lib/i18n";
import { type Action, CopyableId } from "~/shared/ui/actions";
import { JsonViewer, KeyValueList } from "~/shared/ui/data-display";
import { DetailPanel } from "~/shared/ui/layout";
import { Badge, Link, Tabs, TabsList, TabsTrigger } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { type AuditEvent, EVENT_HASH, PREV_HASH } from "../data/audit";
import { decisionBadge } from "../data/states";

function copyJson(): readonly [Action] {
  return [
    {
      label: i18n._(msg`Копировать JSON`),
      run: { onSelect: () => undefined },
    },
  ];
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
            {
              label: t`Действие`,
              value: (
                <span className="flex flex-col leading-tight">
                  <span className="font-mono text-caption">{event.action.code}</span>
                  <span className="text-caption text-muted-foreground">{event.action.label}</span>
                </span>
              ),
            },
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
          {RELATED.length}
          {")"}
        </h3>
        <ul className="m-0 flex list-none flex-col gap-1 p-0">
          {RELATED.map(([code, time]) => (
            <li key={code}>
              <Link href={`/audit/${code}`} className="text-brand underline">
                {code}
                {" · "}
                {time}
              </Link>
            </li>
          ))}
        </ul>
      </section>
    </div>
  );
}

export function EventPanel({
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
