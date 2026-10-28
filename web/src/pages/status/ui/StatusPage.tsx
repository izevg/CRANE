import { useLingui } from "@lingui/react/macro";
import { useQuery } from "@tanstack/react-query";
import { getGetStatusQueryOptions } from "~/shared/api";
import { setAutoRefreshPaused, useAutoRefreshPaused } from "~/shared/lib/state";
import { AutoRefreshToggle } from "~/shared/ui/actions";
import { PageHeader } from "~/shared/ui/layout";
import { Skeleton } from "~/shared/ui/primitives";
import { RelativeTime, StatusBadge } from "~/shared/ui/status";
import { MODE_STATUS } from "../model/labels";
import { ComponentsCard } from "./ComponentsCard";
import { DegradedCallout, RefreshFailed } from "./Notices";
import { StatusTiles } from "./StatusTiles";

/** E15 «Статус системы»: архетип D (упрощённый). Данные и опрос — те же опции, что в loader маршрута. */
export function StatusPage() {
  const { t } = useLingui();
  const paused = useAutoRefreshPaused();
  const { data, error, dataUpdatedAt, isError, refetch } = useQuery(getGetStatusQueryOptions());
  if (data === undefined) {
    if (error !== null) {
      // данных нет и получить не удалось: показывает граница ошибки маршрута, а не вечный скелетон
      throw error;
    }
    // Данные кладёт в кэш loader маршрута; пусто бывает только если запись успели вытеснить.
    return <Skeleton aria-busy="true" className="h-64 w-full" />;
  }
  const status = data.data;
  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t`Статус системы`}
        status={<StatusBadge {...MODE_STATUS[status.mode]} />}
        aside={
          <>
            <span className="text-caption text-muted-foreground">
              {t`Обновлено`} <RelativeTime ts={dataUpdatedAt} />
            </span>
            <AutoRefreshToggle paused={paused} onPausedChange={setAutoRefreshPaused} />
          </>
        }
      />
      {isError ? <RefreshFailed onRetry={refetch} /> : null}
      <DegradedCallout codes={status.degraded ?? []} />
      <StatusTiles status={status} {...(isError ? { staleSince: dataUpdatedAt } : {})} />
      <ComponentsCard components={status.components ?? []} />
    </div>
  );
}
