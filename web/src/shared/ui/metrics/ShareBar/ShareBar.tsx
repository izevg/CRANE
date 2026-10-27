// i18n: не мигрирован
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

type ShareItem = {
  id: string;
  label: string;
  value: number;
  /** Значение для легенды в единицах вызывающего: «4,2 ТБ». */
  display: string;
};

type ShareBarProps = WithClassName<{
  title: string;
  /** Заголовки колонок легенды: «команда» / «формат» переключаемы вызывающим. */
  columns: { name: string; value: string; share: string };
  items: readonly ShareItem[];
  /** Для сворачиваемого остатка «Прочие»: формат суммы (по умолчанию — число). */
  formatOther?: (sum: number) => string;
  caption?: ReactNode;
}>;

/** Сколько крупнейших долей показываем отдельно; остальное сворачивается в «Прочие (N)». */
const TOP_LIMIT = 8;
const OTHER_ID = "__other__";
const OTHER_LABEL = "Прочие";

/** Цвета сегментов — `--chart-1..5` по кругу; «Прочие» — нейтральный. */
const PALETTE = ["bg-chart-1", "bg-chart-2", "bg-chart-3", "bg-chart-4", "bg-chart-5"] as const;
const OTHER_COLOR = "bg-muted-foreground";

type Segment = ShareItem & { color: string };

function toSegments(items: readonly ShareItem[], formatOther: (sum: number) => string): Segment[] {
  const sorted = [...items].sort((a, b) => b.value - a.value);
  const top = sorted.slice(0, TOP_LIMIT).map((item, index) => ({
    ...item,
    color: PALETTE[index % PALETTE.length] ?? OTHER_COLOR,
  }));
  const rest = sorted.slice(TOP_LIMIT);
  if (rest.length === 0) {
    return top;
  }
  const sum = rest.reduce((total, item) => total + item.value, 0);
  return [
    ...top,
    {
      id: OTHER_ID,
      label: `${OTHER_LABEL} (${rest.length})`,
      value: sum,
      display: formatOther(sum),
      color: OTHER_COLOR,
    },
  ];
}

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
  const titleId = useId();
  const format = useTextFormat();
  const segments = toSegments(items, formatOther ?? ((sum) => format.number(sum)));
  const total = segments.reduce((sum, segment) => sum + segment.value, 0);
  const share = (value: number) => (total > 0 ? value / total : 0);

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
                {format.percent(share(segment.value))}
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {caption ? <div className="text-caption text-muted-foreground">{caption}</div> : null}
    </figure>
  );
}
