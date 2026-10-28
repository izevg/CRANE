import { cx } from "~/shared/lib/core";

export type ButtonVariant = "primary" | "secondary" | "ghost" | "destructive";
export type TextSize = "default" | "sm";
export type IconSize = "icon" | "icon-sm";
type ButtonSize = TextSize | IconSize;

const BASE =
  "group/button inline-flex shrink-0 select-none items-center justify-center gap-2 whitespace-nowrap rounded-md border border-transparent text-body font-semibold transition-[background-color,color,transform] duration-(--dur-fast) ease-(--ease-out) motion-safe:active:scale-[0.97] data-disabled:cursor-not-allowed data-disabled:active:scale-100 aria-disabled:cursor-not-allowed aria-disabled:active:scale-100 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4";

// Отключённое состояние задаётся и через data-disabled (Base UI), и через aria-disabled
// (кнопка с причиной остаётся фокусируемой, поэтому не `disabled`).
const VARIANT = {
  primary:
    "bg-primary text-primary-foreground hover:bg-primary-hover active:bg-primary-active data-disabled:bg-surface-high data-disabled:text-muted-foreground data-disabled:hover:bg-surface-high aria-disabled:bg-surface-high aria-disabled:text-muted-foreground aria-disabled:hover:bg-surface-high",
  secondary:
    "bg-secondary text-secondary-foreground hover:bg-surface-high active:bg-surface-highest data-disabled:bg-surface-high data-disabled:text-muted-foreground data-disabled:hover:bg-surface-high aria-disabled:bg-surface-high aria-disabled:text-muted-foreground aria-disabled:hover:bg-surface-high",
  ghost:
    "bg-transparent text-foreground hover:bg-accent active:bg-surface-high data-disabled:bg-transparent data-disabled:text-muted-foreground data-disabled:hover:bg-transparent aria-disabled:bg-transparent aria-disabled:text-muted-foreground aria-disabled:hover:bg-transparent",
  // деструктивное действие: текст error-тона без заливки (правило кита)
  destructive:
    "bg-transparent text-error hover:bg-error-bg active:bg-error-bg data-disabled:bg-transparent data-disabled:text-muted-foreground data-disabled:hover:bg-transparent aria-disabled:bg-transparent aria-disabled:text-muted-foreground aria-disabled:hover:bg-transparent",
} satisfies Record<ButtonVariant, string>;

// Ни один размер не меньше 24 × 24 (WCAG 2.5.8).
const SIZE = {
  default: "h-9 px-4",
  sm: "h-8 px-3",
  icon: "size-8",
  "icon-sm": "size-6",
} satisfies Record<ButtonSize, string>;

type StyleOptions = { variant?: ButtonVariant; size?: ButtonSize };

/** Классы кнопки для элементов, которые не могут быть `<button>` (ссылка, выглядящая как кнопка). */
export function buttonStyles({ variant = "primary", size = "default" }: StyleOptions = {}): string {
  return cx(BASE, VARIANT[variant], SIZE[size]);
}
