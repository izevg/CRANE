export type ShareItem = {
  id: string;
  label: string;
  value: number;
  /** Значение для легенды в единицах вызывающего: «4,2 ТБ». */
  display: string;
};

/** Сколько крупнейших долей показываем отдельно; остальное сворачивается в «Прочие (N)». */
const TOP_LIMIT = 8;
const OTHER_ID = "__other__";

/** Цвета сегментов — `--chart-1..5` по кругу; «Прочие» — нейтральный. */
const PALETTE = ["bg-chart-1", "bg-chart-2", "bg-chart-3", "bg-chart-4", "bg-chart-5"] as const;
const OTHER_COLOR = "bg-muted-foreground";

export type Segment = ShareItem & { color: string };

/**
 * Крупнейшие доли по убыванию; хвост сворачивается в одну строку «Прочие (N)»: подпись и формат суммы
 * приходят от вызывающего (язык и единицы — не забота расчёта).
 */
export function toSegments(
  items: readonly ShareItem[],
  formatOther: (sum: number) => string,
  otherLabel: (count: number) => string,
): Segment[] {
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
      label: otherLabel(rest.length),
      value: sum,
      display: formatOther(sum),
      color: OTHER_COLOR,
    },
  ];
}

/** Доля сегмента от суммы всех; пустая сумма даёт 0. */
export function shareOf(value: number, total: number): number {
  return total > 0 ? value / total : 0;
}
