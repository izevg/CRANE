// biome-ignore-all lint/suspicious/noDeprecatedImports: помечена устаревшей только перегрузка t(i18n); форма ``t`…` `` из макроса Lingui 6 актуальна
import { plural, t } from "@lingui/core/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { useCallback, useId, useMemo, useState } from "react";
import { expect, within } from "storybook/test";
import { type Action, CopyableId } from "~/shared/ui/actions";
import { CodeBlock, columnHelper, DataTable } from "~/shared/ui/data-display";
import { SecretOnceDialog } from "~/shared/ui/dialogs";
import { SegmentedControl } from "~/shared/ui/inputs";
import { PageHeader } from "~/shared/ui/layout";
import {
  Button,
  Callout,
  CalloutDescription,
  Checkbox,
  Field,
  FieldDescription,
  FieldLabel,
  Input,
  Link,
  Popover,
  PopoverContent,
  PopoverTitle,
  PopoverTrigger,
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
  Textarea,
} from "~/shared/ui/primitives";
import { EmptyState, StatusBadge } from "~/shared/ui/status";
import { ACTIVE_TOKENS, REVOKED_TOKENS, TOKEN_LIMIT, TOKENS, type TokenRow } from "../data/tokens";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, screenParameters } from "../frame/screen";

const SECRET = "crp_a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6e7f8a9b0c1d2e3f4";
const EXPIRY_WARNING_DAYS = 14;
const UNUSED_WARNING_DAYS = 30;

type Scope = "active" | "revoked" | "all";

function segments() {
  return [
    {
      value: "active",
      label: t`Активные`,
      count: ACTIVE_TOKENS,
    },
    {
      value: "revoked",
      label: t`Отозванные`,
      count: REVOKED_TOKENS,
    },
    {
      value: "all",
      label: t`Все`,
      count: TOKENS.length,
    },
  ] as const;
}

function ttlSegments() {
  return [
    {
      value: "7",
      label: t`7 дней`,
    },
    {
      value: "30",
      label: t`30 дней`,
    },
    {
      value: "90",
      label: t`90 дней`,
    },
  ] as const;
}

const REPOS = ["npm-hosted", "pypi-hosted", "maven-central-proxy", "nuget-hosted"] as const;

const MUTED = "text-muted-foreground";

function expiryCell(row: TokenRow) {
  if (row.revoked !== undefined) {
    return <span className={MUTED}>{t`Отозван ${row.revoked}`}</span>;
  }
  if (row.expiresInDays === null) {
    return <span>{t`Не истекает`}</span>;
  }
  if (row.expiresInDays <= EXPIRY_WARNING_DAYS) {
    return (
      <StatusBadge
        tone="warning"
        label={t`${plural(row.expiresInDays, { one: "через # день", few: "через # дня", many: "через # дней", other: "через # дня" })}`}
      />
    );
  }
  return (
    <span>
      {t`${plural(row.expiresInDays, { one: "через # день", few: "через # дня", many: "через # дней", other: "через # дня" })}`}
    </span>
  );
}

function lastUsedCell(row: TokenRow) {
  const days = row.lastUsedDaysAgo;
  const muted = row.revoked === undefined ? "" : MUTED;
  if (days === null) {
    return <span className={muted || MUTED}>{t`Не использовался`}</span>;
  }
  if (days >= UNUSED_WARNING_DAYS && row.revoked === undefined) {
    return (
      <StatusBadge
        tone="warning"
        label={t`${plural(days, { one: "не использовался # день", few: "не использовался # дня", many: "не использовался # дней", other: "не использовался # дня" })}`}
      />
    );
  }
  return (
    <span className={muted}>
      {days === 0
        ? t`Сегодня`
        : t`${plural(days, { one: "# день назад", few: "# дня назад", many: "# дней назад", other: "# дня назад" })}`}
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
        <PopoverTitle>{t`Права токена ${row.name}`}</PopoverTitle>
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

const columns = helper.columns([
  helper.accessor("name", {
    header: () => t`Название`,
    meta: { priority: 1 },
    cell: (info) => (
      <span className={info.row.original.revoked === undefined ? "font-semibold" : MUTED}>
        {info.getValue()}
      </span>
    ),
  }),
  helper.accessor("kind", {
    header: () => t`Тип`,
    meta: { priority: 2 },
    cell: (info) => (
      <span className={info.row.original.revoked === undefined ? "" : MUTED}>
        {info.getValue() === "session" ? t`Сессионный · 12 ч` : t`Долгоживущий`}
      </span>
    ),
  }),
  helper.accessor("prefix", {
    header: () => t`Префикс`,
    meta: { priority: 1 },
    cell: (info) => (
      <CopyableId value={info.getValue()} full label={t`Скопировать префикс токена`} />
    ),
  }),
  helper.accessor("scopes", {
    header: () => t`Права`,
    meta: { priority: 2 },
    cell: (info) => scopesCell(info.row.original),
  }),
  helper.accessor("created", {
    header: () => t`Создан`,
    meta: { priority: 2 },
    cell: (info) => (
      <span className={info.row.original.revoked === undefined ? "" : MUTED}>
        {info.getValue()}
      </span>
    ),
  }),
  helper.display({
    id: "expires",
    header: () => t`Истекает`,
    meta: { priority: 2 },
    cell: (info) => expiryCell(info.row.original),
  }),
  helper.display({
    id: "lastUsed",
    header: () => t`Последнее использование`,
    meta: { priority: 2 },
    cell: (info) => lastUsedCell(info.row.original),
  }),
]);

const getRowId = (row: TokenRow) => row.id;

function matches(row: TokenRow, scope: Scope, query: string): boolean {
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

// Спека волны 2: screens S5 — будущий features/tokens/NewTokenSheet (форма в Sheet)
function NewTokenSheet({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (v: boolean) => void;
}) {
  const [ttl, setTtl] = useState("30");
  const close = useCallback(() => onOpenChange(false), [onOpenChange]);
  const rightsId = useId();
  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent>
        <SheetHeader>
          <SheetTitle>{t`Новый токен`}</SheetTitle>
          <SheetDescription>{t`Секрет будет показан один раз после выпуска`}</SheetDescription>
        </SheetHeader>
        <SheetBody className="flex flex-col gap-4">
          <Field name="name">
            <FieldLabel>{t`Название`}</FieldLabel>
            <Input defaultValue="ci-platform · npm" />
            <FieldDescription>{t`До 64 символов, видно только вам`}</FieldDescription>
          </Field>
          <fieldset aria-labelledby={rightsId} className="flex flex-col gap-2">
            <legend id={rightsId} className="text-body font-semibold">
              {t`Права по репозиториям`}
            </legend>
            {REPOS.map((repo) => (
              <div
                key={repo}
                className="flex flex-wrap items-center justify-between gap-x-4 gap-y-1"
              >
                <span className="text-body">{repo}</span>
                <span className="flex items-center gap-3 text-caption">
                  <span className="flex items-center gap-1.5">
                    <Checkbox aria-label={t`${repo}: чтение`} defaultChecked />
                    read
                  </span>
                  <span className="flex items-center gap-1.5">
                    <Checkbox aria-label={t`${repo}: публикация`} />
                    publish
                  </span>
                </span>
              </div>
            ))}
          </fieldset>
          <div className="flex flex-col gap-1.5">
            <p className="text-body font-semibold">{t`Срок действия`}</p>
            <SegmentedControl
              label={t`Срок действия`}
              items={ttlSegments()}
              value={ttl}
              onValueChange={setTtl}
            />
            <p className="text-caption text-muted-foreground">
              {t`Максимум 90 дней — ограничено политикой`}
            </p>
          </div>
          <Field name="description">
            <FieldLabel>{t`Описание`}</FieldLabel>
            <Textarea defaultValue={t`Публикация npm-пакетов из GitLab CI проекта platform/ui`} />
          </Field>
        </SheetBody>
        <SheetFooter>
          <Button>{t`Выпустить`}</Button>
          <Button variant="secondary" onClick={close}>
            {t`Отмена`}
          </Button>
        </SheetFooter>
      </SheetContent>
    </Sheet>
  );
}

// Спека волны 2: screens S5 — будущий pages/tokens + widgets/tokens-table
function TokensTable({ scope, query }: { scope: Scope; query: string }) {
  const data = useMemo(() => TOKENS.filter((row) => matches(row, scope, query)), [scope, query]);
  const actions = useCallback(
    (row: TokenRow): readonly Action[] =>
      row.revoked === undefined
        ? [{ label: t`Отозвать`, run: { onSelect: () => undefined }, destructive: true }]
        : [],
    [],
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
  return (
    <div className="flex flex-col items-center gap-4 rounded-xl bg-card py-6 shadow-card">
      <EmptyState
        variant="first-run"
        title={t`Токенов пока нет`}
        description={t`Токен нужен пакетным менеджерам и CLI. Для входа из терминала хватит cctl login.`}
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

function Screen(props: Props) {
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

const meta = {
  title: "Экраны/S5 Мои токены",
  component: Screen,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof Screen>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DesktopLight: Story = {
  name: "Desktop 1440 · светлая",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "Мои токены" })).toBeVisible();
    await expect(canvas.getByRole("table", { name: "Токены доступа" })).toBeVisible();
  },
};

export const DesktopDark: Story = {
  name: "Desktop 1440 · тёмная",
  globals: { ...desktop, theme: "dark" },
};

export const Mobile: Story = {
  name: "Mobile 390",
  globals: mobile,
};

export const Empty: Story = {
  name: "Пустое состояние",
  args: { empty: true },
};

/** Диалог остаётся открытым в момент axe-проверки. */
export const SecretShown: Story = {
  name: "Секрет показан один раз",
  args: { secret: true },
  play: async () => {
    await expect(
      await within(document.body).findByRole("alertdialog", { name: "Токен создан" }),
    ).toBeVisible();
  },
};

/** Sheet остаётся открытым в момент axe-проверки. */
export const NewTokenSheetOpen: Story = {
  name: "Sheet «Новый токен»",
  args: { sheet: true },
  play: async () => {
    await expect(
      await within(document.body).findByRole("dialog", { name: "Новый токен" }),
    ).toBeVisible();
  },
};

export const English: Story = {
  ...DesktopLight,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "My tokens" })).toBeVisible();
    await expect(canvas.getByRole("navigation", { name: "Primary menu" })).toBeVisible();
  },
};
