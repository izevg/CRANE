import { useLingui } from "@lingui/react/macro";
import { useSuspenseQuery } from "@tanstack/react-query";
import { getGetStatusSuspenseQueryOptions } from "~/shared/api";
import { autoRefreshPaused } from "~/shared/lib/state";
import { AutoRefreshToggle } from "~/shared/ui/actions";
import { PageHeader } from "~/shared/ui/layout";
import { StatusBadge } from "~/shared/ui/status";
import { MODE_STATUS } from "../model/labels";
import { ComponentsCard } from "./ComponentsCard";
import { DegradedCallout, RefreshFailed, UpdatedAt } from "./Notices";
import { StatusTiles } from "./StatusTiles";

/** E15 «Статус системы»: архетип D (упрощённый). Данные и опрос — те же опции, что в loader маршрута. */
export function StatusPage() {
  const { t } = useLingui();
  const paused = autoRefreshPaused.use();
  // Данные кладёт в кэш loader маршрута, поэтому `data` определён; ошибка первой загрузки уходит в границу маршрута,
  // а ошибка очередного опроса при наличии данных не бросается и приходит как `isError`.
  const { data, dataUpdatedAt, errorUpdatedAt, isError, isFetching, refetch } = useSuspenseQuery(
    getGetStatusSuspenseQueryOptions(),
  );
  const status = data.data;
  return (
    <div className="flex flex-col gap-6">
      <PageHeader
        title={t`Статус системы`}
        status={<StatusBadge {...MODE_STATUS[status.mode]} />}
        aside={
          <>
            <UpdatedAt ts={dataUpdatedAt} />
            <AutoRefreshToggle paused={paused} onPausedChange={autoRefreshPaused.set} />
          </>
        }
      />
      {isError ? (
        <RefreshFailed onRetry={refetch} retrying={isFetching} failedAt={errorUpdatedAt} />
      ) : null}
      <DegradedCallout codes={status.degraded ?? []} />
      <StatusTiles status={status} {...(isError ? { staleSince: dataUpdatedAt } : {})} />
      <ComponentsCard components={status.components ?? []} />
    </div>
  );
}
