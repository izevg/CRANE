import type * as React from "react";
import { cx } from "~/shared/lib/core";
import { TONE_SURFACE, type Tone, toneGlyph } from "../tone/tone";

type CalloutTone = Exclude<Tone, "neutral">;

type CalloutProps = Omit<React.ComponentProps<"div">, "role"> & {
  tone: CalloutTone;
  /** `alert` — срочное, прерывает чтение; по умолчанию `status` (вежливое объявление). */
  urgent?: boolean;
};

/** Встроенное сообщение в потоке страницы (inline-callout, основа баннера). */
export function Callout({ tone, urgent = false, className, children, ...props }: CalloutProps) {
  const Icon = toneGlyph(tone);
  return (
    <div
      data-slot="callout"
      data-tone={tone}
      role={urgent ? "alert" : "status"}
      className={cx(
        "flex w-full gap-2 rounded-md px-3 py-2 text-left text-body",
        TONE_SURFACE[tone],
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
