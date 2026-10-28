import type { TextFormat } from "~/shared/lib/text";

type SeriesPoint = { t: number; v: number };

export type Series = {
  id: string;
  label: string;
  points: readonly SeriesPoint[];
  /** Пунктирный штрих: ряд отличается не только цветом (WCAG 1.4.1). */
  dash?: boolean;
};

export type Slo = { value: number; label: string };

/** Строка объединённых рядов: время и значения по id серии (нет точки — нет ключа). */
export type Row = { t: number } & Record<string, number>;

const HOUR_MS = 3_600_000;
const DAY_MS = 24 * HOUR_MS;

const tickFormats = new Map<
  string,
  { clock: Intl.DateTimeFormat; dayAndClock: Intl.DateTimeFormat }
>();

function ticksFor(locale: string) {
  let cached = tickFormats.get(locale);
  if (cached === undefined) {
    cached = {
      clock: new Intl.DateTimeFormat(locale, { hour: "2-digit", minute: "2-digit" }),
      dayAndClock: new Intl.DateTimeFormat(locale, {
        day: "numeric",
        month: "short",
        hour: "2-digit",
        minute: "2-digit",
      }),
    };
    tickFormats.set(locale, cached);
  }
  return cached;
}

/** Объединяет ряды по времени: у каждой серии свои метки, у графика и таблицы — общая ось. */
export function mergeSeries(series: readonly Series[]): Row[] {
  const byTime = new Map<number, Row>();
  for (const { id, points } of series) {
    for (const { t, v } of points) {
      const row = byTime.get(t) ?? { t };
      row[id] = v;
      byTime.set(t, row);
    }
  }
  return [...byTime.values()].sort((a, b) => a.t - b.t);
}

/** Ось X в локальном времени; если окно шире суток, к часам добавляется дата. */
export function makeTickFormatter(rows: readonly Row[], locale: string): (t: number) => string {
  const first = rows[0]?.t ?? 0;
  const last = rows.at(-1)?.t ?? 0;
  const { clock, dayAndClock } = ticksFor(locale);
  const format = last - first > DAY_MS ? dayAndClock : clock;
  return (t) => format.format(new Date(t));
}

/** Значение с единицей для подсказки и таблицы. */
export function formatValueWithUnit(
  value: number,
  unit: string | undefined,
  format: TextFormat,
): string {
  const text = format.number(value);
  return unit ? `${text} ${unit}` : text;
}

/** Индекс последней строки, где у серии есть значение: туда ставится прямая подпись. */
export function lastIndexOf(rows: readonly Row[], id: string): number {
  return rows.findLastIndex((row) => id in row);
}
