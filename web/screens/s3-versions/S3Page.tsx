import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useCallback, useId, useMemo, useState } from "react";
import { i18n } from "~/shared/lib/i18n";
import { DataTable } from "~/shared/ui/data-display";
import { FilterSelect, SegmentedControl } from "~/shared/ui/inputs";
import { PageHeader } from "~/shared/ui/layout";
import { StatChip } from "~/shared/ui/metrics";
import { Badge, Input, Switch, Tabs, TabsList, TabsTrigger } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { STATE_COUNTS, VERSION_ROWS, type VersionRow } from "../data/versions";
import { ScreenFrame } from "../frame/ScreenFrame";
import { columns, getRowId, noop, rowActions, type Segment, segments } from "./S3Columns";
import { VersionPanel } from "./S3Panel";

function matches(row: VersionRow, segment: Segment, query: string): boolean {
  return (
    (segment === "all" || row.state === segment) &&
    row.version.toLowerCase().includes(query.trim().toLowerCase())
  );
}

type ScreenProps = { initialPanelOpen?: boolean };

// Спека волны 2: архетипы A + C (composition §4) — будущий widget `PackageVersions`.
export function S3Screen({ initialPanelOpen = true }: ScreenProps) {
  const { t } = useLingui();
  const [segment, setSegment] = useState<Segment>("all");
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>("v-1.4.2");
  const [panelOpen, setPanelOpen] = useState(initialPanelOpen);
  const [sort, setSort] = useState<string | null>(null);
  const preId = useId();
  const searchId = useId();
  const rows = useMemo(
    () => VERSION_ROWS.filter((row) => matches(row, segment, query)),
    [segment, query],
  );
  const select = useCallback((id: string) => {
    setSelectedId(id);
    setPanelOpen(true);
  }, []);
  const changeSegment = useCallback((next: Segment) => setSegment(next), []);
  const changeQuery = useCallback(
    (event: { target: { value: string } }) => setQuery(event.target.value),
    [],
  );
  return (
    <ScreenFrame
      active="packages"
      breadcrumbs={{
        trail: [
          { label: t`Мои пакеты`, href: "/packages" },
          { label: "npm-hosted", href: "/repositories/npm-hosted" },
        ],
        current: "@acme/ui",
      }}
    >
      <div className="flex flex-col gap-6">
        <PageHeader
          title="@acme/ui"
          status={<StatusBadge tone="success" label={t`Активен`} />}
          chips={
            <>
              <Badge variant="outline">{"npm"}</Badge>
              <Badge variant="outline">{"npm-hosted · hosted"}</Badge>
            </>
          }
          caption={i18n._(
            msg`Библиотека интерфейсных компонентов команды web. Владелец: Иванов И.`,
          )}
          stats={
            <>
              <StatChip label={t`Версий`} value={STATE_COUNTS.all} />
              <StatChip label={t`Последняя`} value={<span className="font-mono">{"1.4.2"}</span>} />
              <StatChip label={t`Загрузок за 30 дн`} value="12 340" />
            </>
          }
          tabs={
            <Tabs value="versions">
              <TabsList variant="line" aria-label={t`Разделы пакета`}>
                <TabsTrigger value="overview">{t`Обзор`}</TabsTrigger>
                <TabsTrigger value="versions">{t`Версии`}</TabsTrigger>
                <TabsTrigger value="risks">{t`Риски`}</TabsTrigger>
                <TabsTrigger value="members">{t`Участники`}</TabsTrigger>
              </TabsList>
            </Tabs>
          }
        />
        <div className="flex items-start gap-6">
          <section aria-label={t`Версии`} className="flex min-w-0 flex-1 flex-col gap-4">
            <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
              <SegmentedControl
                label={t`Состояние версий`}
                items={segments()}
                value={segment}
                onValueChange={changeSegment}
              />
              <div className="flex items-center gap-2">
                <Switch id={preId} size="sm" />
                <label htmlFor={preId} className="text-body">
                  {"Pre-release"}
                </label>
              </div>
              <FilterSelect
                label={t`Сортировка`}
                allLabel={t`Сначала новые`}
                value={sort}
                options={[
                  { value: "published", label: t`Сначала старые` },
                  { value: "downloads", label: t`По загрузкам` },
                ]}
                onValueChange={setSort}
              />
              <Input
                id={searchId}
                type="search"
                aria-label={t`Поиск по версии или тегу`}
                placeholder={t`Поиск по версии или тегу`}
                className="max-w-64"
                value={query}
                onChange={changeQuery}
              />
            </div>
            <DataTable
              columns={columns}
              data={[...rows]}
              getRowId={getRowId}
              caption={t`Версии @acme/ui`}
              compact={panelOpen}
              selectedId={selectedId}
              onRowSelect={select}
              actions={rowActions}
              loadMore={{ label: t`Загрузить ещё`, run: { onSelect: noop } }}
              empty={
                <p className="p-6 text-center text-muted-foreground">
                  {t`Версий с такими условиями нет`}
                </p>
              }
            />
          </section>
          <VersionPanel open={panelOpen} onOpenChange={setPanelOpen} />
        </div>
      </div>
    </ScreenFrame>
  );
}
