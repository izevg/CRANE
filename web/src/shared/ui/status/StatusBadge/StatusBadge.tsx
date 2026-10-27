import { OctagonAlertIcon, TriangleAlertIcon } from "lucide-react";
import type { ComponentType, SVGProps } from "react";
import { cx } from "~/shared/lib/core";
import { usePrefersReducedMotion } from "~/shared/lib/state";

export type StatusTone = "success" | "info" | "warning" | "error" | "neutral";

type Glyph = ComponentType<SVGProps<SVGSVGElement>>;

const BADGE = {
  success: "bg-success-bg text-success",
  info: "bg-info-bg text-info",
  warning: "bg-warning-bg text-warning",
  error: "bg-error-bg text-error",
  neutral: "bg-muted text-muted-foreground",
} satisfies Record<StatusTone, string>;

const DOT = {
  success: "bg-success-strong",
  info: "bg-info-strong",
  warning: "bg-warning-strong",
  error: "bg-error-strong",
  neutral: "bg-muted-foreground",
} satisfies Record<StatusTone, string>;

const RING = {
  success: "after:bg-success-strong",
  info: "after:bg-info-strong",
  warning: "after:bg-warning-strong",
  error: "after:bg-error-strong",
  neutral: "after:bg-muted-foreground",
} satisfies Record<StatusTone, string>;

/**
 * Статус не передаётся только цветом (WCAG 1.4.1): у warning и error вместо точки стоит иконка,
 * и это выводится из тона, а не выбирается вызывающим.
 */
const TONE_ICON = {
  success: null,
  info: null,
  warning: TriangleAlertIcon,
  error: OctagonAlertIcon,
  neutral: null,
} satisfies Record<StatusTone, Glyph | null>;

type StatusBadgeProps = {
  tone: StatusTone;
  /** Текст обязателен: статус не передаётся одним цветом. */
  label: string;
  /** Пульсация только у выполняющихся процессов; при reduced motion выключена. */
  pulse?: boolean;
};

export function StatusBadge({ tone, label, pulse = false }: StatusBadgeProps) {
  const reducedMotion = usePrefersReducedMotion();
  const animated = pulse && !reducedMotion;
  const Icon = TONE_ICON[tone];
  return (
    <span
      data-tone={tone}
      className={cx(
        "inline-flex h-6 items-center gap-1.5 rounded-full border border-transparent px-2.5 text-caption font-semibold",
        BADGE[tone],
      )}
    >
      <span
        aria-hidden="true"
        data-pulse={animated ? "true" : undefined}
        className={cx(
          "relative inline-flex shrink-0 items-center justify-center",
          animated &&
            cx(
              "after:absolute after:inset-0 after:animate-status-pulse after:rounded-full",
              RING[tone],
            ),
        )}
      >
        {Icon === null ? (
          <span
            data-slot="status-dot"
            className={cx("size-2 rounded-full forced-color-adjust-none", DOT[tone])}
          />
        ) : (
          <Icon data-slot="status-icon" className="size-3.5" />
        )}
      </span>
      {label}
    </span>
  );
}
