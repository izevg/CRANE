import type { TextFormat } from "~/shared/lib/text";

export type TileDelta = {
  /** Процентное изменение, например -3.1. */
  pct: number;
  /** Какое направление считается хорошим. */
  polarity: "up-good" | "down-good" | "neutral";
};

export type TileValue = number | { count: number; capped: boolean } | string | null;

/** `toneLabel` обязателен при не нейтральном тоне: цвет один не передаёт статус (WCAG 1.4.1). */
export type TileTone = { tone?: "neutral" } | { tone: "warning" | "error"; toneLabel: string };

export type ActiveTone = "warning" | "error";

const MINUS = "−";
const UNIT_SEPARATOR = " ";
const PERCENT = 100;

export function resolveTone(props: TileTone): { tone: ActiveTone; label: string } | null {
  if (!("toneLabel" in props)) {
    return null;
  }
  return { tone: props.tone, label: props.toneLabel };
}

/** Хорошее ли изменение: решает направление и полярность метрики; нулевое и нейтральное — ни то ни другое. */
export function deltaVerdict({ pct, polarity }: TileDelta): "good" | "bad" | "neutral" {
  if (polarity === "neutral" || pct === 0) {
    return "neutral";
  }
  const good = polarity === "up-good" ? pct > 0 : pct < 0;
  return good ? "good" : "bad";
}

/** Изменение со знаком: «−3,1 %». Знак минус — настоящий, а не дефис. */
export function deltaText(delta: TileDelta, format: TextFormat): string {
  const sign = delta.pct < 0 ? MINUS : "";
  return `${sign}${format.percent(Math.abs(delta.pct) / PERCENT)}`;
}

export function formatValue(value: Exclude<TileValue, null>, format: TextFormat): string {
  if (typeof value === "number") {
    return format.number(value);
  }
  return typeof value === "string" ? value : format.count(value);
}

/** Значение с единицей для имени плитки-ссылки; `null` — данных нет (подпись подбирает UI). */
export function valueWithUnit(
  value: TileValue,
  unit: string | undefined,
  format: TextFormat,
): string | null {
  if (value === null) {
    return null;
  }
  const number = formatValue(value, format);
  return unit ? `${number}${UNIT_SEPARATOR}${unit}` : number;
}

export const UNIT_GAP = UNIT_SEPARATOR;
