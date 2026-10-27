import type * as React from "react";
import { cx } from "~/shared/lib/core";

/** Заглушка загрузки: форму и размер задаёт вызывающий через `className`. Скрыта от скринридера. */
export function Skeleton({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="skeleton"
      aria-hidden="true"
      className={cx("rounded-md bg-muted motion-safe:animate-pulse", className)}
      {...props}
    />
  );
}
