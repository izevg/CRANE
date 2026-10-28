import { useLingui } from "@lingui/react/macro";
import type { Status } from "~/shared/api";
import { MetricTile } from "~/shared/ui/metrics";
import { lagOf } from "../model/lag";

type Props = {
  status: Status;
  /** Время последних удачных данных, если очередное обновление не удалось: плитки показывают их возраст. */
  staleSince?: number;
};

/** Ряд плиток: версия и отставание фидов и экспорта в SIEM. Плитка без данных показывает «—», а не 0. */
export function StatusTiles({ status, staleSince }: Props) {
  const { t } = useLingui();
  const stale =
    staleSince === undefined ? {} : { stale: { updatedAt: new Date(staleSince).toISOString() } };
  const units = { s: t`с`, min: t`мин` };
  const feeds = lagOf(status.feeds_max_lag_s);
  const siem = lagOf(status.siem_export_lag_s);
  return (
    <div className="grid grid-cols-[repeat(auto-fit,minmax(200px,1fr))] gap-4">
      <MetricTile label={t`Версия`} value={status.version} {...stale} />
      <MetricTile
        label={t`Отставание фидов`}
        value={feeds?.value ?? null}
        {...(feeds ? { unit: units[feeds.unit] } : {})}
        {...stale}
      />
      <MetricTile
        label={t`Отставание экспорта в SIEM`}
        value={siem?.value ?? null}
        {...(siem ? { unit: units[siem.unit] } : {})}
        {...stale}
      />
    </div>
  );
}
