import type * as React from "react";
import { cx } from "~/shared/lib/core";

type DivProps = React.ComponentProps<"div">;

export function Card({ className, ...props }: DivProps) {
  return (
    <div
      data-slot="card"
      className={cx(
        "group/card flex flex-col gap-(--card-spacing) overflow-hidden rounded-xl bg-card py-(--card-spacing) text-body text-card-foreground shadow-card [--card-spacing:--spacing(4)] has-data-[slot=card-footer]:pb-0",
        className,
      )}
      {...props}
    />
  );
}

export function CardHeader({ className, ...props }: DivProps) {
  return (
    <div
      data-slot="card-header"
      className={cx(
        "@container/card-header grid auto-rows-min items-start gap-1 px-(--card-spacing) has-data-[slot=card-action]:grid-cols-[1fr_auto]",
        className,
      )}
      {...props}
    />
  );
}

export function CardTitle({ className, ...props }: DivProps) {
  return <div data-slot="card-title" className={cx("text-h3", className)} {...props} />;
}

export function CardAction({ className, ...props }: DivProps) {
  return (
    <div
      data-slot="card-action"
      className={cx("col-start-2 row-span-2 row-start-1 self-start justify-self-end", className)}
      {...props}
    />
  );
}

export function CardContent({ className, ...props }: DivProps) {
  return (
    <div data-slot="card-content" className={cx("px-(--card-spacing)", className)} {...props} />
  );
}

export function CardFooter({ className, ...props }: DivProps) {
  return (
    <div
      data-slot="card-footer"
      className={cx("flex items-center border-t bg-muted p-(--card-spacing)", className)}
      {...props}
    />
  );
}
