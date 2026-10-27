import { cx } from "~/shared/lib/core";
import { useNow } from "~/shared/lib/state";
import { formatDateTime, formatRelative } from "~/shared/lib/text";

type RelativeTimeProps = {
  ts: string | number | Date;
  className?: string;
};

/**
 * «3 мин назад» с полным временем в `title` и машиночитаемым `dateTime`.
 * Обновляется по общему минутному таймеру; живым регионом не является (изменения не объявляются).
 */
export function RelativeTime({ ts, className }: RelativeTimeProps) {
  const now = useNow();
  const time = ts instanceof Date ? ts.getTime() : new Date(ts).getTime();
  const iso = Number.isNaN(time) ? undefined : new Date(time).toISOString();
  return (
    <time dateTime={iso} title={formatDateTime(ts)} className={cx("tabular-nums", className)}>
      {formatRelative(ts, now)}
    </time>
  );
}
