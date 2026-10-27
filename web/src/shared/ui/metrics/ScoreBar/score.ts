import type { TextFormat } from "~/shared/lib/text";

const MIN = 0;
const MAX = 1;
const FRACTION_DIGITS = 2;
const FRACTION = { minimumFractionDigits: FRACTION_DIGITS, maximumFractionDigits: FRACTION_DIGITS };

export const SCORE_MIN = MIN;
export const SCORE_MAX = MAX;

/** Значение вне диапазона 0..1 обрезается, нечисло считается нулём. */
function clamp(value: number): number {
  return Number.isFinite(value) ? Math.min(Math.max(value, MIN), MAX) : MIN;
}

/** Оценка и уверенность модели строками с двумя знаками; `confidence` — `null`, если не передана. */
export function scoreTexts(
  value: number,
  confidence: number | undefined,
  text: TextFormat,
): { score: number; shown: string; confidence: string | null } {
  const score = clamp(value);
  const format = (n: number): string => text.number(n, FRACTION);
  return {
    score,
    shown: format(score),
    confidence: confidence === undefined ? null : format(clamp(confidence)),
  };
}
