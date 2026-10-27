import { msg } from "@lingui/core/macro";
import { VersionFlags, VersionNumber, VersionStatus } from "~/entities/version";
import { i18n } from "~/shared/lib/i18n";
import type { Action } from "~/shared/ui/actions";
import { columnHelper } from "~/shared/ui/data-display";
import { STATE_COUNTS, type VersionRow } from "../data/versions";
import { Moment } from "../frame/Moment";

/** Маршрут SPA страницы экстренной блокировки: страница строит его из `block_id`, а не берёт `href` API. */
function blockLink({ block_reason: reason }: VersionRow): { blockHref?: string } {
  return reason?.kind === "emergency" ? { blockHref: `/emergency-blocks/${reason.block_id}` } : {};
}

/** Момент публикации выбранной версии: тот же, что первое событие ленты, таблицы и сведений. */
export const SELECTED_AT = "2026-10-27T13:42:00Z";

const helper = columnHelper<VersionRow>();

export const columns = helper.columns([
  helper.accessor("version", {
    header: () => i18n._(msg`Версия`),
    meta: { priority: 1 },
    cell: (info) => <VersionNumber name={info.getValue()} state={info.row.original.state} />,
  }),
  helper.accessor("state", {
    header: () => i18n._(msg`Состояние`),
    meta: { priority: 1 },
    cell: (info) => <VersionStatus version={info.row.original} {...blockLink(info.row.original)} />,
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

export const getRowId = (row: VersionRow) => row.id;

export function segments() {
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

export type Segment = ReturnType<typeof segments>[number]["value"];

export const rowActions = (row: VersionRow): readonly Action[] =>
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

export const noop = () => undefined;
