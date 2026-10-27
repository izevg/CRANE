import { Meter } from "@base-ui/react/meter";
import { cx, type WithClassName } from "~/shared/lib/core";
import { formatNumber } from "~/shared/lib/text";
import { Badge } from "~/shared/ui/primitives";

type ScoreBarProps = WithClassName<{
  /** Подпись оценки, уже переведённая вызывающим («Релевантность эксплуатации»). */
  label: string;
  /** 0..1; значения вне диапазона обрезаются. */
  value: number;
  /** Уверенность модели 0..1. */
  confidence?: number;
  /** Главные факторы оценки. */
  factors?: readonly string[];
  /** Модель-источник: «systemone v3.1». */
  model?: string;
  /** Теневая модель: оценка не влияет на решения. */
  shadow?: boolean;
}>;

/**
 * Пороги тона полосы (общие для кита): ниже LOW — info, ниже HIGH — warning, иначе error.
 * У оценок systemone высокое значение — больший риск, поэтому success не используется вообще:
 * зелёный сказал бы «безопасно» там, где модель этого не утверждает.
 */
const LOW_THRESHOLD = 0.5;
const HIGH_THRESHOLD = 0.8;
/** Ниже этой уверенности оценка не участвует в политике (разд. 11.6). */
const MIN_CONFIDENCE = LOW_THRESHOLD;

const MIN = 0;
const MAX = 1;
const FRACTION_DIGITS = 2;

const CONFIDENCE_PREFIX = "уверенность ";
const MODEL_PREFIX = "модель ";
const SHADOW = "shadow";
const SHADOW_NOTE = "не влияет на решения";
const LOW_CONFIDENCE_NOTE = "низкая уверенность, не влияет на политику";
const VALUE_TEXT_SEPARATOR = ", ";

type BarTone = "info" | "warning" | "error" | "muted";

const FILL = {
  info: "bg-info-strong",
  warning: "bg-warning-strong",
  error: "bg-error-strong",
  muted: "bg-muted-foreground",
} satisfies Record<BarTone, string>;

function format(value: number): string {
  return formatNumber(value, {
    minimumFractionDigits: FRACTION_DIGITS,
    maximumFractionDigits: FRACTION_DIGITS,
  });
}

function clamp(value: number): number {
  return Number.isFinite(value) ? Math.min(Math.max(value, MIN), MAX) : MIN;
}

function scoreTone(value: number): BarTone {
  if (value < LOW_THRESHOLD) {
    return "info";
  }
  return value < HIGH_THRESHOLD ? "warning" : "error";
}

/**
 * Оценка модели 0..1 на Base UI `Meter` (роль `meter`, `aria-valuetext` «0,88, уверенность 0,92»).
 * Полоса одного тона, без градиентов. Низкая уверенность и shadow-модель делают полосу приглушённой:
 * такая оценка не влияет на решения, и цвет не должен намекать на вердикт.
 */
export function ScoreBar({
  label,
  value,
  confidence,
  factors = [],
  model,
  shadow = false,
  className,
}: ScoreBarProps) {
  const score = clamp(value);
  const lowConfidence = confidence !== undefined && confidence < MIN_CONFIDENCE;
  const tone: BarTone = lowConfidence || shadow ? "muted" : scoreTone(score);
  const shown = format(score);
  const confidenceText = confidence === undefined ? null : format(clamp(confidence));
  const valueText =
    confidenceText === null
      ? shown
      : `${shown}${VALUE_TEXT_SEPARATOR}${CONFIDENCE_PREFIX}${confidenceText}`;

  return (
    <div
      data-slot="score-bar"
      data-shadow={shadow || undefined}
      className={cx("flex flex-col gap-1.5", className)}
    >
      <Meter.Root
        value={score}
        min={MIN}
        max={MAX}
        aria-valuetext={valueText}
        className="flex flex-col gap-1"
      >
        <div className="flex items-baseline justify-between gap-2">
          <Meter.Label className="min-w-0 truncate text-body">{label}</Meter.Label>
          <span
            aria-hidden="true"
            className={cx(
              "text-body font-semibold tabular-nums",
              shadow ? "text-muted-foreground" : "text-foreground",
            )}
          >
            {shown}
          </span>
        </div>
        <Meter.Track className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
          <Meter.Indicator data-tone={tone} className={cx("h-full rounded-full", FILL[tone])} />
        </Meter.Track>
      </Meter.Root>
      {confidenceText === null ? null : (
        <span className="text-caption text-muted-foreground tabular-nums">
          {CONFIDENCE_PREFIX}
          {confidenceText}
        </span>
      )}
      {factors.length > 0 ? (
        <ul className="flex flex-wrap gap-1">
          {factors.map((factor) => (
            <li key={factor}>
              <Badge variant="secondary">{factor}</Badge>
            </li>
          ))}
        </ul>
      ) : null}
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1 text-caption text-muted-foreground">
        {model === undefined ? null : (
          <span>
            {MODEL_PREFIX}
            {model}
          </span>
        )}
        {shadow ? (
          <>
            <Badge variant="outline">{SHADOW}</Badge>
            <span>{SHADOW_NOTE}</span>
          </>
        ) : null}
        {lowConfidence ? <span>{LOW_CONFIDENCE_NOTE}</span> : null}
      </div>
    </div>
  );
}
