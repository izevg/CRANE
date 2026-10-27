import { lazy, Suspense, useCallback, useId, useState } from "react";
import { cx, type WithClassName } from "~/shared/lib/core";
import { formatDateTime } from "~/shared/lib/text";
import {
  Button,
  Skeleton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "~/shared/ui/primitives";
import { formatValueWithUnit, mergeSeries, type Series, type Slo } from "./timeseries";

/** Recharts тяжёлый: отдельный чанк, который не попадает в основной бандл. */
const TimeSeriesPlot = lazy(() =>
  import("./TimeSeriesPlot").then((module) => ({ default: module.TimeSeriesPlot })),
);

type TimeSeriesChartProps = WithClassName<{
  title: string;
  /** Текстовая сводка графика (WCAG 1.1.1): «p95 за 24 ч: 84 мс, максимум 138 мс в 21:40». */
  summary: string;
  series: readonly Series[];
  /** Порог SLO: пунктирная линия с подписью у правого края. */
  slo?: Slo;
  unit?: string;
  /** Заливка под линией; только для одного ряда, при нескольких игнорируется. */
  area?: boolean;
}>;

const SHOW_TABLE = "Показать таблицей";
const SHOW_CHART = "Показать графиком";
const TIME_COLUMN = "Время";
const NO_VALUE = "—";
/** Высота графика одна для заглушки и самого графика: раскладка не прыгает при загрузке чанка. */
const PLOT_HEIGHT = "h-60";

function DataTable({ series, unit }: { series: readonly Series[]; unit: string | undefined }) {
  const rows = mergeSeries(series);
  return (
    <Table>
      <TableHeader>
        <TableRow>
          <TableHead>{TIME_COLUMN}</TableHead>
          {series.map((item) => (
            <TableHead key={item.id} className="text-right">
              {item.label}
            </TableHead>
          ))}
        </TableRow>
      </TableHeader>
      <TableBody>
        {rows.map((row) => (
          <TableRow key={row.t}>
            <TableCell>{formatDateTime(row.t)}</TableCell>
            {series.map((item) => {
              const value = row[item.id];
              return (
                <TableCell key={item.id} className="text-right tabular-nums">
                  {value === undefined ? NO_VALUE : formatValueWithUnit(value, unit)}
                </TableCell>
              );
            })}
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

/**
 * Временной ряд: `figure` с заголовком и текстовой сводкой, график без сглаживания и анимации,
 * кнопка «Показать таблицей» с теми же данными. Оболочка лёгкая; Recharts подгружается лениво.
 */
export function TimeSeriesChart({
  title,
  summary,
  series,
  slo,
  unit,
  area = false,
  className,
}: TimeSeriesChartProps) {
  const titleId = useId();
  const [tabular, setTabular] = useState(false);
  const toggleLabel = tabular ? SHOW_CHART : SHOW_TABLE;
  const toggle = useCallback(() => setTabular((value) => !value), []);

  return (
    <figure
      data-slot="time-series-chart"
      aria-labelledby={titleId}
      className={cx("flex flex-col gap-3", className)}
    >
      <div className="flex items-start justify-between gap-2">
        <figcaption id={titleId} className="text-h3">
          {title}
        </figcaption>
        <Button variant="ghost" size="sm" onClick={toggle}>
          {toggleLabel}
        </Button>
      </div>
      <p className="text-caption text-muted-foreground">{summary}</p>
      {tabular ? (
        <DataTable series={series} unit={unit} />
      ) : (
        <div data-slot="time-series-plot" className={PLOT_HEIGHT}>
          <Suspense fallback={<Skeleton className="size-full" />}>
            <TimeSeriesPlot series={series} slo={slo} unit={unit} area={area} />
          </Suspense>
        </div>
      )}
    </figure>
  );
}
