import {
  CircleCheckIcon,
  InfoIcon,
  type LucideIcon,
  OctagonAlertIcon,
  TriangleAlertIcon,
} from "lucide-react";

/** Тон статуса. Единый для бейджей, callout, тостов, плиток и строк уведомлений. */
export type Tone = "success" | "info" | "warning" | "error" | "neutral";

/**
 * Статус не передаётся только цветом (WCAG 1.4.1): у warning и error вместо точки стоит иконка.
 * `null` — тон, который показывается точкой.
 */
export const TONE_ICON = {
  success: null,
  info: null,
  warning: TriangleAlertIcon,
  error: OctagonAlertIcon,
  neutral: null,
} satisfies Record<Tone, LucideIcon | null>;

const FALLBACK_GLYPH = {
  success: CircleCheckIcon,
  info: InfoIcon,
  neutral: InfoIcon,
} satisfies Record<Exclude<Tone, "warning" | "error">, LucideIcon>;

/** Иконка тона для компонентов, где значок нужен всегда (callout, тост, строка уведомления). */
export function toneGlyph(tone: Tone): LucideIcon {
  return TONE_ICON[tone] ?? FALLBACK_GLYPH[tone as keyof typeof FALLBACK_GLYPH];
}

/** Цвет текста тона. */
export const TONE_TEXT = {
  success: "text-success",
  info: "text-info",
  warning: "text-warning",
  error: "text-error",
  neutral: "text-muted-foreground",
} satisfies Record<Tone, string>;

/** Контейнер тона: фон и текст. */
export const TONE_SURFACE = {
  success: "bg-success-bg text-success",
  info: "bg-info-bg text-info",
  warning: "bg-warning-bg text-warning",
  error: "bg-error-bg text-error",
  neutral: "bg-muted text-muted-foreground",
} satisfies Record<Tone, string>;

/** Насыщенная заливка: точка, полоса, маркер ленты. */
export const TONE_STRONG = {
  success: "bg-success-strong",
  info: "bg-info-strong",
  warning: "bg-warning-strong",
  error: "bg-error-strong",
  neutral: "bg-muted-foreground",
} satisfies Record<Tone, string>;

/** Кольцо пульсации вокруг точки или иконки. */
export const TONE_RING = {
  success: "after:bg-success-strong",
  info: "after:bg-info-strong",
  warning: "after:bg-warning-strong",
  error: "after:bg-error-strong",
  neutral: "after:bg-muted-foreground",
} satisfies Record<Tone, string>;

/** Цвет линии в SVG (графики рисуются не классами). */
export const TONE_STROKE = {
  success: "var(--success-strong)",
  info: "var(--info-strong)",
  warning: "var(--warning-strong)",
  error: "var(--error-strong)",
  neutral: "var(--muted-foreground)",
} satisfies Record<Tone, string>;
