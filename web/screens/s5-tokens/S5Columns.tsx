import { msg, plural } from "@lingui/core/macro";
import { i18n } from "~/shared/lib/i18n";
import { CopyableId } from "~/shared/ui/actions";
import { columnHelper } from "~/shared/ui/data-display";
import {
  Button,
  Popover,
  PopoverContent,
  PopoverTitle,
  PopoverTrigger,
} from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import type { TokenRow } from "../data/tokens";

const EXPIRY_WARNING_DAYS = 14;
const UNUSED_WARNING_DAYS = 30;

export type Scope = "active" | "revoked" | "all";

const MUTED = "text-muted-foreground";

function expiryCell(row: TokenRow) {
  if (row.revoked !== undefined) {
    return <span className={MUTED}>{i18n._(msg`Отозван ${row.revoked}`)}</span>;
  }
  if (row.expiresInDays === null) {
    return <span>{i18n._(msg`Не истекает`)}</span>;
  }
  if (row.expiresInDays <= EXPIRY_WARNING_DAYS) {
    return (
      <StatusBadge
        tone="warning"
        label={i18n._(
          msg`${plural(row.expiresInDays, { one: "через # день", few: "через # дня", many: "через # дней", other: "через # дня" })}`,
        )}
      />
    );
  }
  return (
    <span>
      {i18n._(
        msg`${plural(row.expiresInDays, { one: "через # день", few: "через # дня", many: "через # дней", other: "через # дня" })}`,
      )}
    </span>
  );
}

function lastUsedCell(row: TokenRow) {
  const days = row.lastUsedDaysAgo;
  const muted = row.revoked === undefined ? "" : MUTED;
  if (days === null) {
    return <span className={muted || MUTED}>{i18n._(msg`Не использовался`)}</span>;
  }
  if (days >= UNUSED_WARNING_DAYS && row.revoked === undefined) {
    return (
      <StatusBadge
        tone="warning"
        label={i18n._(
          msg`${plural(days, { one: "не использовался # день", few: "не использовался # дня", many: "не использовался # дней", other: "не использовался # дня" })}`,
        )}
      />
    );
  }
  return (
    <span className={muted}>
      {days === 0
        ? i18n._(msg`Сегодня`)
        : i18n._(
            msg`${plural(days, { one: "# день назад", few: "# дня назад", many: "# дней назад", other: "# дня назад" })}`,
          )}
    </span>
  );
}

// Спека волны 2: screens S5 — сводка прав с Popover, будущий features/tokens/ScopesSummary
function scopesCell(row: TokenRow) {
  return (
    <Popover>
      <PopoverTrigger
        render={
          <Button variant="ghost" size="sm" className={row.revoked === undefined ? "" : MUTED} />
        }
      >
        {row.scopes}
      </PopoverTrigger>
      <PopoverContent>
        <PopoverTitle>{i18n._(msg`Права токена ${row.name}`)}</PopoverTitle>
        <ul className="list-disc space-y-0.5 pl-4 text-body">
          {row.scopeList.map((scope) => (
            <li key={scope}>{scope}</li>
          ))}
        </ul>
      </PopoverContent>
    </Popover>
  );
}

const helper = columnHelper<TokenRow>();

export const columns = helper.columns([
  helper.accessor("name", {
    header: () => i18n._(msg`Название`),
    meta: { priority: 1 },
    cell: (info) => (
      <span className={info.row.original.revoked === undefined ? "font-semibold" : MUTED}>
        {info.getValue()}
      </span>
    ),
  }),
  helper.accessor("kind", {
    header: () => i18n._(msg`Тип`),
    meta: { priority: 2 },
    cell: (info) => (
      <span className={info.row.original.revoked === undefined ? "" : MUTED}>
        {info.getValue() === "session" ? i18n._(msg`Сессионный · 12 ч`) : i18n._(msg`Долгоживущий`)}
      </span>
    ),
  }),
  helper.accessor("prefix", {
    header: () => i18n._(msg`Префикс`),
    meta: { priority: 1 },
    cell: (info) => (
      <CopyableId value={info.getValue()} full label={i18n._(msg`Скопировать префикс токена`)} />
    ),
  }),
  helper.accessor("scopes", {
    header: () => i18n._(msg`Права`),
    meta: { priority: 2 },
    cell: (info) => scopesCell(info.row.original),
  }),
  helper.accessor("created", {
    header: () => i18n._(msg`Создан`),
    meta: { priority: 2 },
    cell: (info) => (
      <span className={info.row.original.revoked === undefined ? "" : MUTED}>
        {info.getValue()}
      </span>
    ),
  }),
  helper.display({
    id: "expires",
    header: () => i18n._(msg`Истекает`),
    meta: { priority: 2 },
    cell: (info) => expiryCell(info.row.original),
  }),
  helper.display({
    id: "lastUsed",
    header: () => i18n._(msg`Последнее использование`),
    meta: { priority: 2 },
    cell: (info) => lastUsedCell(info.row.original),
  }),
]);

export const getRowId = (row: TokenRow) => row.id;

export function matches(row: TokenRow, scope: Scope, query: string): boolean {
  const inScope =
    scope === "all" ||
    (scope === "revoked" ? row.revoked !== undefined : row.revoked === undefined);
  const needle = query.trim().toLowerCase();
  return (
    inScope &&
    (needle === "" ||
      row.name.toLowerCase().includes(needle) ||
      row.prefix.toLowerCase().includes(needle))
  );
}
