import { cx } from "~/shared/lib/core";
import { TONE_ICON, TONE_RING, TONE_STRONG, TONE_SURFACE, type Tone } from "../../primitives";

type StatusBadgeProps = {
  tone: Tone;
  /** Текст обязателен: статус не передаётся одним цветом. */
  label: string;
  /** Пульсация только у выполняющихся процессов; при prefers-reduced-motion выключена в CSS. */
  pulse?: boolean;
};

export function StatusBadge({ tone, label, pulse = false }: StatusBadgeProps) {
  const Icon = TONE_ICON[tone];
  return (
    <span
      data-tone={tone}
      className={cx(
        "inline-flex h-6 items-center gap-1.5 rounded-full border border-transparent px-2.5 text-caption font-semibold",
        TONE_SURFACE[tone],
      )}
    >
      <span
        aria-hidden="true"
        data-pulse={pulse ? "true" : undefined}
        className={cx(
          "relative inline-flex shrink-0 items-center justify-center",
          // prefers-reduced-motion отключает пульсацию в CSS (motion-safe), JS о нём не знает
          pulse &&
            cx(
              "after:absolute after:inset-0 after:rounded-full motion-safe:after:animate-status-pulse",
              TONE_RING[tone],
            ),
        )}
      >
        {Icon === null ? (
          <span
            data-slot="status-dot"
            className={cx("size-2 rounded-full forced-color-adjust-none", TONE_STRONG[tone])}
          />
        ) : (
          <Icon data-slot="status-icon" className="size-3.5" />
        )}
      </span>
      {label}
    </span>
  );
}
