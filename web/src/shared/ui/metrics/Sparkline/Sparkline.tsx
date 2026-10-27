import { cx, type WithClassName } from "~/shared/lib/core";

type SparklineTone = "neutral" | "warning" | "error";

const COLOR = {
  neutral: "var(--chart-1)",
  warning: "var(--warning-strong)",
  error: "var(--error-strong)",
} satisfies Record<SparklineTone, string>;

const WIDTH = 100;
const HEIGHT = 32;
const PADDING = 2;
const AREA_OPACITY = "0.08";
const LINE_WIDTH = "1.5";
const REFERENCE_WIDTH = "1";
const DASH = "3 3";
const MIN_POINTS = 2;

type SparklineProps = WithClassName<{
  values: readonly number[];
  /** Пунктирная линия цели (SLO). */
  reference?: number | undefined;
  tone?: SparklineTone | undefined;
}>;

/**
 * Мини-график без осей: линия, лёгкая заливка и пунктир цели. Рисуется чистым SVG и не анимируется.
 * Декоративный: смысл тренда дублирует дельта текстом, поэтому от скринридера скрыт.
 * Меньше двух точек — пунктир-заглушка.
 */
export function Sparkline({ values, reference, tone = "neutral", className }: SparklineProps) {
  const color = COLOR[tone];
  const box = `0 0 ${WIDTH} ${HEIGHT}`;
  const classes = cx("size-full", className);

  if (values.length < MIN_POINTS) {
    return (
      <svg aria-hidden="true" viewBox={box} preserveAspectRatio="none" className={classes}>
        <line
          x1="0"
          x2={WIDTH}
          y1={HEIGHT / 2}
          y2={HEIGHT / 2}
          stroke="var(--chart-grid)"
          strokeWidth={LINE_WIDTH}
          strokeDasharray={DASH}
          vectorEffect="non-scaling-stroke"
        />
      </svg>
    );
  }

  const all = reference === undefined ? values : [...values, reference];
  const min = Math.min(...all);
  const span = Math.max(...all) - min || 1;
  const x = (index: number) => (index / (values.length - 1)) * WIDTH;
  const y = (value: number) => HEIGHT - PADDING - ((value - min) / span) * (HEIGHT - PADDING * 2);
  const line = values
    .map(
      (value, index) => `${index === 0 ? "M" : "L"}${x(index).toFixed(2)} ${y(value).toFixed(2)}`,
    )
    .join(" ");

  return (
    <svg aria-hidden="true" viewBox={box} preserveAspectRatio="none" className={classes}>
      <path
        d={`${line} L${WIDTH} ${HEIGHT} L0 ${HEIGHT} Z`}
        fill={color}
        fillOpacity={AREA_OPACITY}
      />
      {reference === undefined ? null : (
        <line
          x1="0"
          x2={WIDTH}
          y1={y(reference)}
          y2={y(reference)}
          stroke="var(--chart-slo)"
          strokeWidth={REFERENCE_WIDTH}
          strokeDasharray={DASH}
          vectorEffect="non-scaling-stroke"
        />
      )}
      <path
        d={line}
        fill="none"
        stroke={color}
        strokeWidth={LINE_WIDTH}
        strokeLinejoin="round"
        vectorEffect="non-scaling-stroke"
      />
    </svg>
  );
}
