import { useLingui } from "@lingui/react/macro";
import { type ReactNode, useId } from "react";
import { cx, type WithClassName } from "~/shared/lib/core";
import { useTextFormat } from "~/shared/lib/i18n";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "~/shared/ui/primitives";
import { type ShareItem, shareOf, toSegments } from "./share";

type ShareBarProps = WithClassName<{
  title: string;
  /** Заголовки колонок легенды: «команда» / «формат» переключаемы вызывающим. */
  columns: { name: string; value: string; share: string };
  items: readonly ShareItem[];
  /** Для сворачиваемого остатка «Прочие»: формат суммы (по умолчанию — число). */
  formatOther?: (sum: number) => string;
  caption?: ReactNode;
}>;

/**
 * Доля от целого: одна горизонтальная полоса 100 % и легенда-таблица — главный носитель данных.
 * Полоса декоративна (`aria-hidden`), сегменты разделены зазором 2 px цвета карточки, потому что
 * соседние цвета палитры различаются меньше, чем на 1,3:1.
 */
export function ShareBar({
  title,
  columns,
  items,
  formatOther,
  caption,
  className,
}: ShareBarProps) {
  const { t } = useLingui();
  const titleId = useId();
  const format = useTextFormat();
  const segments = toSegments(
    items,
    formatOther ?? ((sum) => format.number(sum)),
    (count) => t`Прочие (${count})`,
  );
  const total = segments.reduce((sum, segment) => sum + segment.value, 0);

  return (
    <figure
      data-slot="share-bar"
      aria-labelledby={titleId}
      className={cx("flex flex-col gap-3", className)}
    >
      <figcaption id={titleId} className="text-h3">
        {title}
      </figcaption>
      <div aria-hidden="true" className="flex h-3 w-full gap-0.5 overflow-hidden rounded-full">
        {segments.map((segment) => (
          <span
            key={segment.id}
            style={{ flexGrow: segment.value }}
            className={cx("h-full min-w-px basis-0", segment.color)}
          />
        ))}
      </div>
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>{columns.name}</TableHead>
            <TableHead className="text-right">{columns.value}</TableHead>
            <TableHead className="text-right">{columns.share}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {segments.map((segment) => (
            <TableRow key={segment.id}>
              <TableCell>
                <span className="inline-flex items-center gap-2">
                  <span
                    aria-hidden="true"
                    className={cx("size-2.5 shrink-0 rounded-full", segment.color)}
                  />
                  {segment.label}
                </span>
              </TableCell>
              <TableCell className="text-right tabular-nums">{segment.display}</TableCell>
              <TableCell className="text-right tabular-nums">
                {format.percent(shareOf(segment.value, total))}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {caption ? <div className="text-caption text-muted-foreground">{caption}</div> : null}
    </figure>
  );
}
