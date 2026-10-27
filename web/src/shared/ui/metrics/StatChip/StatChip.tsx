import type { ReactNode } from "react";
import { cx, type WithClassName } from "~/shared/lib/core";

/** Пробел между подписью и значением: без него скринридер прочитает «Версий48» одним словом. */
const SEPARATOR = " ";

type StatChipProps = WithClassName<{
  label: string;
  value: ReactNode;
}>;

/**
 * Факт в шапке страницы («Версий 48»). Не ссылка: если число ведёт в список, это вкладка со счётчиком.
 * Порядок «подпись → значение» одной текстовой строкой — так его читает скринридер.
 */
export function StatChip({ label, value, className }: StatChipProps) {
  return (
    <span
      data-slot="stat-chip"
      className={cx(
        "inline-flex h-7 items-center gap-1 rounded-full bg-muted px-3 text-caption text-muted-foreground",
        className,
      )}
    >
      <span>{label}</span>
      {SEPARATOR}
      <span className="font-bold text-foreground tabular-nums">{value}</span>
    </span>
  );
}
