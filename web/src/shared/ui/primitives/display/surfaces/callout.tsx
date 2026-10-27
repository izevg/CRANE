import { CircleCheckIcon, InfoIcon, OctagonAlertIcon, TriangleAlertIcon } from "lucide-react";
import type * as React from "react";
import { cx } from "~/shared/lib/core";

type CalloutTone = "info" | "success" | "warning" | "error";

const TONE = {
  info: "bg-info-bg text-info",
  success: "bg-success-bg text-success",
  warning: "bg-warning-bg text-warning",
  error: "bg-error-bg text-error",
} satisfies Record<CalloutTone, string>;

// Иконка есть у каждого тона, а у warning и error — обязательно: статус не передаётся только цветом (WCAG 1.4.1).
const TONE_ICON = {
  info: InfoIcon,
  success: CircleCheckIcon,
  warning: TriangleAlertIcon,
  error: OctagonAlertIcon,
} satisfies Record<CalloutTone, React.ComponentType<React.ComponentProps<"svg">>>;

type CalloutProps = Omit<React.ComponentProps<"div">, "role"> & {
  tone: CalloutTone;
  /** `alert` — срочное, прерывает чтение; по умолчанию `status` (вежливое объявление). */
  urgent?: boolean;
};

/** Встроенное сообщение в потоке страницы (inline-callout, основа баннера). */
export function Callout({ tone, urgent = false, className, children, ...props }: CalloutProps) {
  const Icon = TONE_ICON[tone];
  return (
    <div
      data-slot="callout"
      data-tone={tone}
      role={urgent ? "alert" : "status"}
      className={cx(
        "flex w-full gap-2 rounded-md px-3 py-2 text-left text-body",
        TONE[tone],
        className,
      )}
      {...props}
    >
      <Icon aria-hidden="true" className="mt-0.5 size-4 shrink-0" />
      <div className="min-w-0 flex-1">{children}</div>
    </div>
  );
}

export function CalloutTitle({ className, ...props }: React.ComponentProps<"div">) {
  return <div data-slot="callout-title" className={cx("font-semibold", className)} {...props} />;
}

export function CalloutDescription({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="callout-description"
      className={cx(
        "text-body [&_a]:underline [&_a]:underline-offset-3 [&_p:not(:last-child)]:mb-4",
        className,
      )}
      {...props}
    />
  );
}
