import { Meter } from "@base-ui/react/meter";
import { cx, type WithClassName } from "~/shared/lib/core";
import { formatNumber } from "~/shared/lib/text";
import { Badge, TONE_STRONG, type Tone } from "~/shared/ui/primitives";

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
  /** Тон полосы: решает вызывающий (пороги и смысл оценки — не забота компонента). */
  tone: Tone;
  /** Пояснения под полосой: «низкая уверенность, не влияет на политику». */
  notes?: readonly string[];
  /** Визуально вторичная оценка (например, теневая модель): значение приглушено. */
  secondary?: boolean;
  /** Бейдж рядом с моделью: «shadow». */
  badge?: string;
}>;

const MIN = 0;
const MAX = 1;
const FRACTION_DIGITS = 2;

const CONFIDENCE_PREFIX = "уверенность ";
const MODEL_PREFIX = "модель ";
const VALUE_TEXT_SEPARATOR = ", ";

function format(value: number): string {
  return formatNumber(value, {
    minimumFractionDigits: FRACTION_DIGITS,
    maximumFractionDigits: FRACTION_DIGITS,
  });
}

function clamp(value: number): number {
  return Number.isFinite(value) ? Math.min(Math.max(value, MIN), MAX) : MIN;
}

/**
 * Оценка модели 0..1 на Base UI `Meter` (роль `meter`, `aria-valuetext` «0,88, уверенность 0,92»).
 * Полоса одного тона, без градиентов. Компонент ничего не знает о доменных порогах: тон, пояснения
 * и признак вторичности приходят готовыми.
 */
export function ScoreBar({
  label,
  value,
  confidence,
  factors = [],
  model,
  tone,
  notes = [],
  secondary = false,
  badge,
  className,
}: ScoreBarProps) {
  const score = clamp(value);
  const shown = format(score);
  const confidenceText = confidence === undefined ? null : format(clamp(confidence));
  const valueText =
    confidenceText === null
      ? shown
      : `${shown}${VALUE_TEXT_SEPARATOR}${CONFIDENCE_PREFIX}${confidenceText}`;

  return (
    <div
      data-slot="score-bar"
      data-secondary={secondary || undefined}
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
              secondary ? "text-muted-foreground" : "text-foreground",
            )}
          >
            {shown}
          </span>
        </div>
        <Meter.Track className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
          <Meter.Indicator
            data-tone={tone}
            className={cx("h-full rounded-full", TONE_STRONG[tone])}
          />
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
        {badge === undefined ? null : <Badge variant="outline">{badge}</Badge>}
        {notes.map((note) => (
          <span key={note}>{note}</span>
        ))}
      </div>
    </div>
  );
}
