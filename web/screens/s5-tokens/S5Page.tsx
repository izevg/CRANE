import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useCallback, useMemo, useState } from "react";
import { i18n } from "~/shared/lib/i18n";
import type { Action } from "~/shared/ui/actions";
import { CodeBlock, DataTable } from "~/shared/ui/data-display";
import { SecretOnceDialog } from "~/shared/ui/dialogs";
import { SegmentedControl } from "~/shared/ui/inputs";
import { PageHeader } from "~/shared/ui/layout";
import { Callout, CalloutDescription, Input, Link } from "~/shared/ui/primitives";
import { EmptyState } from "~/shared/ui/status";
import { ACTIVE_TOKENS, REVOKED_TOKENS, TOKEN_LIMIT, TOKENS, type TokenRow } from "../data/tokens";
import { ScreenFrame } from "../frame/ScreenFrame";
import { columns, getRowId, matches, type Scope } from "./S5Columns";
import { NewTokenSheet } from "./S5Sheet";

const SECRET = "crp_a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4";

function segments() {
  return [
    {
      value: "active",
      label: i18n._(msg`Активные`),
      count: ACTIVE_TOKENS,
    },
    {
      value: "revoked",
      label: i18n._(msg`Отозванные`),
      count: REVOKED_TOKENS,
    },
    {
      value: "all",
      label: i18n._(msg`Все`),
      count: TOKENS.length,
    },
  ] as const;
}

// Спека волны 2: screens S5 — будущий pages/tokens + widgets/tokens-table
function TokensTable({ scope, query }: { scope: Scope; query: string }) {
  const { t } = useLingui();
  const data = useMemo(() => TOKENS.filter((row) => matches(row, scope, query)), [scope, query]);
  const actions = useCallback(
    (row: TokenRow): readonly Action[] =>
      row.revoked === undefined
        ? [{ label: t`Отозвать`, run: { onSelect: () => undefined }, destructive: true }]
        : [],
    [t],
  );
  return (
    <DataTable
      columns={columns}
      data={data}
      getRowId={getRowId}
      caption={t`Токены доступа`}
      actions={actions}
      empty={<p className="p-6 text-center text-muted-foreground">{t`Токены не найдены`}</p>}
    />
  );
}

function EmptyTokens({ onIssue }: { onIssue: () => void }) {
  const { t } = useLingui();
  return (
    <div className="flex flex-col items-center gap-4 rounded-xl bg-card py-6 shadow-card">
      <EmptyState
        variant="first-run"
        title={t`Токенов пока нет`}
        description={i18n._(
          msg`Токен нужен пакетным менеджерам и CLI. Для входа из терминала хватит cctl login.`,
        )}
        primary={{ label: t`Выпустить токен`, run: { onSelect: onIssue } }}
      />
      <div className="w-full max-w-md px-4">
        <CodeBlock
          code="cctl login"
          copyLabel={t`Копировать команду входа`}
          label={t`Вход из терминала`}
        />
      </div>
    </div>
  );
}

type Props = { empty?: boolean; sheet?: boolean; secret?: boolean };

function S5Page({ empty = false, sheet = false, secret = false }: Props) {
  const { t } = useLingui();
  const [scope, setScope] = useState<Scope>("all");
  const [query, setQuery] = useState("");
  const [sheetOpen, setSheetOpen] = useState(sheet);
  const [secretShown, setSecretShown] = useState<string | null>(secret ? SECRET : null);
  const issue = useCallback(() => setSheetOpen(true), []);
  const finish = useCallback(() => setSecretShown(null), []);
  const changeScope = useCallback((value: Scope) => setScope(value), []);
  const changeQuery = useCallback(
    (event: React.ChangeEvent<HTMLInputElement>) => setQuery(event.target.value),
    [],
  );
  return (
    <div className="flex flex-col gap-4">
      <PageHeader
        title={t`Мои токены`}
        caption={t`Токены для пакетных менеджеров и CLI. Секрет показывается один раз`}
        primary={{ label: t`Выпустить токен`, run: { onSelect: issue } }}
        secondary={[{ label: t`Мои действия`, run: { href: "/me/actions" } }]}
      />
      <Callout tone="info">
        <CalloutDescription>
          {t`Отзыв применяется за 5 секунд. Для CI используйте `}
          <Link href="/workspaces/platform/trust-policies">{t`политики доверия`}</Link>
          {t` — без секретов.`}
        </CalloutDescription>
      </Callout>
      {empty ? (
        <EmptyTokens onIssue={issue} />
      ) : (
        <>
          <div className="flex flex-wrap items-center gap-x-4 gap-y-3">
            <SegmentedControl
              label={t`Состояние токенов`}
              items={segments()}
              value={scope}
              onValueChange={changeScope}
            />
            <Input
              type="search"
              aria-label={t`Поиск по названию или префиксу`}
              placeholder={t`Название или префикс`}
              value={query}
              onChange={changeQuery}
              className="w-full @lg/page:w-auto @lg/page:max-w-xs @lg/page:flex-1"
            />
            <p className="ml-auto text-caption text-muted-foreground tabular-nums">
              {t`Лимит: ${ACTIVE_TOKENS} из ${TOKEN_LIMIT}`}
            </p>
          </div>
          <TokensTable scope={scope} query={query} />
        </>
      )}
      <NewTokenSheet open={sheetOpen} onOpenChange={setSheetOpen} />
      <SecretOnceDialog secret={secretShown} title={t`Токен создан`} onDone={finish} />
    </div>
  );
}

export function Screen(props: Props) {
  const { t } = useLingui();
  return (
    <ScreenFrame
      active="tokens"
      breadcrumbs={{
        trail: [{ label: t`Настройки`, href: "/settings" }],
        current: t`Мои токены`,
      }}
    >
      <S5Page {...props} />
    </ScreenFrame>
  );
}
