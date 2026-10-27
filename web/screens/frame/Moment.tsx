import { useTextFormat } from "~/shared/lib/i18n";

/** Момент события из ISO-строки в локали интерфейса; тот же вид, что у Timeline: `<time>`, полная дата в `title`. */
export function Moment({ iso }: { iso: string }) {
  const format = useTextFormat();
  return (
    <time dateTime={iso} title={format.dateTime(iso)} className="tabular-nums">
      {format.dateTimeShort(iso)}
    </time>
  );
}
