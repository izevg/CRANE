import { type ReactNode, useCallback } from "react";
import {
  Area,
  CartesianGrid,
  ComposedChart,
  Label,
  Line,
  ReferenceLine,
  ResponsiveContainer,
  Tooltip,
  XAxis,
  YAxis,
} from "recharts";
import { formatDateTime } from "~/shared/lib/text";
import {
  formatValueWithUnit,
  lastIndexOf,
  makeTickFormatter,
  mergeSeries,
  type Series,
  type Slo,
} from "./timeseries";

type PlotProps = {
  series: readonly Series[];
  slo: Slo | undefined;
  unit: string | undefined;
  area: boolean;
};

/** Цвета рядов — токены графиков по кругу (`--chart-1..5`). */
const COLORS = [
  "var(--chart-1)",
  "var(--chart-2)",
  "var(--chart-3)",
  "var(--chart-4)",
  "var(--chart-5)",
] as const;
const FALLBACK_COLOR = "var(--chart-1)";
const DASH = "6 4";
const SOLID = "none";
const SLO_DASH = "4 4";
const STROKE_WIDTH = 2;
const AREA_OPACITY = 0.08;
const LABEL_OFFSET = 8;
const LABEL_WEIGHT = 600;
const MARKER_RADIUS = 4;
const MARKER_GAP = 4;
/** Запас справа под прямые подписи у конца линий. */
const MARGIN = { top: 8, right: 96, bottom: 0, left: 0 };
const TICK = { fill: "var(--muted-foreground)", fontSize: 12 };
const AXIS_LINE = { stroke: "var(--chart-grid)" };
const Y_DOMAIN: [number, "auto"] = [0, "auto"];
const X_DOMAIN: ["dataMin", "dataMax"] = ["dataMin", "dataMax"];

type EndLabelProps = {
  x?: number | string;
  y?: number | string;
  index?: number;
  text: string;
  lastIndex: number;
  color: string;
};

/**
 * Прямая подпись ряда у его последней точки. Текст — `--foreground` (контраст ≥ 4,5:1, WCAG 1.4.3),
 * а цвет ряда вынесен в маркер рядом: `--chart-4/5` как цвет текста на карточке не проходят.
 */
function EndLabel({ x, y, index, text, lastIndex, color }: EndLabelProps) {
  if (index !== lastIndex) {
    return null;
  }
  const cx = Number(x) + LABEL_OFFSET + MARKER_RADIUS;
  return (
    <g>
      <circle cx={cx} cy={Number(y)} r={MARKER_RADIUS} fill={color} />
      <text
        x={cx + MARKER_RADIUS + MARKER_GAP}
        y={Number(y)}
        dominantBaseline="middle"
        fill="var(--foreground)"
        fontSize={TICK.fontSize}
        fontWeight={LABEL_WEIGHT}
      >
        {text}
      </text>
    </g>
  );
}

type SeriesLineProps = { item: Series; color: string; lastIndex: number; filled: boolean };

/** Линия ряда: без сглаживания (`linear`), без точек, с прямой подписью у конца; заливка — только для одного ряда. */
function SeriesLine({ item, color, lastIndex, filled }: SeriesLineProps) {
  const common = {
    dataKey: item.id,
    name: item.label,
    stroke: color,
    strokeWidth: STROKE_WIDTH,
    strokeDasharray: item.dash ? DASH : SOLID,
    type: "linear",
    dot: false,
    connectNulls: true,
    isAnimationActive: false,
    label: <EndLabel text={item.label} lastIndex={lastIndex} color={color} />,
  } as const;
  return filled ? (
    <Area {...common} fill={color} fillOpacity={AREA_OPACITY} />
  ) : (
    <Line {...common} />
  );
}

/** График на Recharts. Грузится отдельным чанком; без анимации, чтобы опрос не «мигал» линиями. */
export function TimeSeriesPlot({ series, slo, unit, area }: PlotProps) {
  const rows = mergeSeries(series);
  const tick = makeTickFormatter(rows);
  const filled = area && series.length === 1;
  const formatTooltipLabel = useCallback((label: ReactNode) => formatDateTime(Number(label)), []);
  const formatTooltipValue = useCallback(
    (value: unknown) => formatValueWithUnit(Number(value), unit),
    [unit],
  );

  return (
    <ResponsiveContainer width="100%" height="100%">
      <ComposedChart data={rows} margin={MARGIN}>
        <CartesianGrid stroke="var(--chart-grid)" vertical={false} />
        <XAxis
          dataKey="t"
          type="number"
          scale="time"
          domain={X_DOMAIN}
          tickFormatter={tick}
          tick={TICK}
          axisLine={AXIS_LINE}
          tickLine={false}
        />
        <YAxis width="auto" domain={Y_DOMAIN} tick={TICK} axisLine={false} tickLine={false} />
        <Tooltip
          isAnimationActive={false}
          labelFormatter={formatTooltipLabel}
          formatter={formatTooltipValue}
        />
        {slo ? (
          <ReferenceLine
            y={slo.value}
            stroke="var(--chart-slo)"
            strokeDasharray={SLO_DASH}
            ifOverflow="extendDomain"
          >
            <Label
              value={slo.label}
              position="insideTopRight"
              fill="var(--muted-foreground)"
              fontSize={TICK.fontSize}
            />
          </ReferenceLine>
        ) : null}
        {series.map((item, position) => (
          <SeriesLine
            key={item.id}
            item={item}
            color={COLORS[position % COLORS.length] ?? FALLBACK_COLOR}
            lastIndex={lastIndexOf(rows, item.id)}
            filled={filled}
          />
        ))}
      </ComposedChart>
    </ResponsiveContainer>
  );
}
