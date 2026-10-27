import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { type ReactNode, useMemo } from "react";
import type { Action } from "~/shared/ui/actions";
import { columnHelper, DataTable } from "~/shared/ui/data-display";
import { PageHeader } from "~/shared/ui/layout";
import { Badge, type Tone } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { PEOPLE, REPOSITORIES } from "../data/common";
import { activeFormat, tr } from "../frame/tr";

type PackageRow = {
  id: string;
  name: string;
  format: string;
  latest: string;
  state: { tone: Tone; label: MessageDescriptor };
  repository: string;
  downloads: number;
  owner: string;
};

const SEEDS = [
  { name: "@acme/ui", format: "npm", latest: "1.4.2", repo: 0 },
  { name: "@acme/ui-tokens", format: "npm", latest: "0.9.14", repo: 0 },
  { name: "lodash", format: "npm", latest: "4.17.21", repo: 1 },
  { name: "axios", format: "npm", latest: "1.7.9", repo: 1 },
  { name: "requests", format: "PyPI", latest: "2.32.3", repo: 2 },
  { name: "acme-billing-client", format: "PyPI", latest: "3.1.0", repo: 2 },
  { name: "org.acme:payments-core", format: "Maven", latest: "7.2.1", repo: 3 },
  {
    name: "com.fasterxml.jackson.core:jackson-databind",
    format: "Maven",
    latest: "2.18.2",
    repo: 3,
  },
  { name: "Acme.Observability", format: "NuGet", latest: "5.0.3", repo: 4 },
  { name: "serde", format: "Cargo", latest: "1.0.217", repo: 5 },
] as const;

const RELEASED: PackageRow["state"] = { tone: "success", label: msg`Выпущена` };

const STATES: readonly PackageRow["state"][] = [
  RELEASED,
  RELEASED,
  { tone: "warning", label: msg`Карантин` },
  { tone: "error", label: msg`Заблокирована` },
];

const ROWS_COUNT = 40;
const DOWNLOADS_BASE = 137;
const DOWNLOADS_STEP = 911;
const DOWNLOADS_SPAN = 48_000;
const PATCH_CYCLE = 7;
const MINOR_CYCLE = 11;

function buildRows(): PackageRow[] {
  return Array.from({ length: ROWS_COUNT }, (_, index) => {
    const seed = SEEDS[index % SEEDS.length] ?? SEEDS[0];
    const round = Math.floor(index / SEEDS.length);
    return {
      id: `pkg-${index}`,
      name:
        round === 0 ? seed.name : `${seed.name}-${["next", "legacy", "core", "cli"][round - 1]}`,
      format: seed.format,
      latest: round === 0 ? seed.latest : `${round}.${index % PATCH_CYCLE}.${index % MINOR_CYCLE}`,
      state: STATES[index % STATES.length] ?? RELEASED,
      repository: REPOSITORIES[seed.repo]?.id ?? "npm-hosted",
      downloads: DOWNLOADS_BASE + ((index * DOWNLOADS_STEP) % DOWNLOADS_SPAN),
      owner: PEOPLE[index % PEOPLE.length] ?? PEOPLE[0],
    };
  });
}

const helper = columnHelper<PackageRow>();
const columns = helper.columns([
  helper.accessor("name", {
    header: () => tr(msg`Пакет`),
    meta: { priority: 1 },
    cell: (info) => <span className="font-semibold">{info.getValue()}</span>,
  }),
  helper.accessor("format", {
    header: () => tr(msg`Формат`),
    meta: { priority: 1 },
    cell: (info) => <Badge variant="outline">{info.getValue()}</Badge>,
  }),
  helper.accessor("latest", { header: () => tr(msg`Последняя`), meta: { priority: 1 } }),
  helper.accessor("state", {
    header: () => tr(msg`Состояние`),
    meta: { priority: 2 },
    cell: (info) => <StatusBadge tone={info.getValue().tone} label={tr(info.getValue().label)} />,
  }),
  helper.accessor("repository", { header: () => tr(msg`Репозиторий`), meta: { priority: 2 } }),
  helper.accessor("downloads", {
    header: () => tr(msg`Загрузок за 30 дн`),
    meta: { priority: 3, numeric: true },
    cell: (info) => activeFormat().number(info.getValue()),
  }),
  helper.accessor("owner", { header: () => tr(msg`Владелец`), meta: { priority: 3 } }),
]);

const getRowId = (row: PackageRow) => row.id;
const publishAction = (): Action => ({
  label: tr(msg`Опубликовать пакет`),
  run: { href: "/help/publish" },
});

type Props = {
  /** Плотная таблица (по умолчанию) или другое содержимое под заголовком. */
  loading?: boolean;
  children?: ReactNode;
};

/**
 * Страница «Мои пакеты» (архетип A, S5/S1): реальный плотный контент под оверлеями S10.
 * Спека волны 2: composition §4A — будущий widgets/packages-table.
 */
export function PackagesBackdrop({ loading = false, children }: Props) {
  const rows = useMemo(buildRows, []);
  return (
    <div className="flex flex-col gap-6 p-4 lg:p-6">
      <PageHeader
        title={tr(msg`Мои пакеты`)}
        caption={tr(msg`Пакеты ваших workspace во всех форматах`)}
        primary={publishAction()}
      />
      {children ?? (
        <DataTable
          columns={columns}
          data={loading ? [] : rows}
          loading={loading}
          getRowId={getRowId}
          caption={tr(msg`Мои пакеты`)}
        />
      )}
    </div>
  );
}
