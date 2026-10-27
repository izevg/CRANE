import type * as React from "react";
import { cx } from "~/shared/lib/core";

/** Клавиша или сочетание в подсказке: «Ctrl K». */
export function Kbd({ className, ...props }: React.ComponentProps<"kbd">) {
  return (
    <kbd
      data-slot="kbd"
      className={cx(
        "pointer-events-none inline-flex h-5 w-fit min-w-5 select-none items-center justify-center gap-1 rounded-sm border border-border bg-muted px-1 font-sans text-caption text-muted-foreground tabular-nums [&_svg:not([class*='size-'])]:size-3",
        className,
      )}
      {...props}
    />
  );
}
