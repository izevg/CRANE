import { ChevronRightIcon } from "lucide-react";
import type * as React from "react";
import { cx } from "~/shared/lib/core";
import { Link, type LinkProps } from "./link";

const LABEL = "Навигационная цепочка";

export function Breadcrumb({ className, ...props }: React.ComponentProps<"nav">) {
  return <nav aria-label={LABEL} data-slot="breadcrumb" className={className} {...props} />;
}

export function BreadcrumbList({ className, ...props }: React.ComponentProps<"ol">) {
  return (
    <ol
      data-slot="breadcrumb-list"
      className={cx(
        "flex min-w-0 flex-wrap items-center gap-1.5 text-body wrap-break-word text-muted-foreground",
        className,
      )}
      {...props}
    />
  );
}

export function BreadcrumbItem({ className, ...props }: React.ComponentProps<"li">) {
  return (
    <li
      data-slot="breadcrumb-item"
      className={cx("inline-flex min-w-0 items-center gap-1", className)}
      {...props}
    />
  );
}

/** Ссылка цепочки идёт через `Link` кита (шов роутера). */
export function BreadcrumbLink({ className, ...props }: LinkProps) {
  return (
    <Link
      data-slot="breadcrumb-link"
      className={cx("transition-colors duration-(--dur-fast) hover:text-foreground", className)}
      {...props}
    />
  );
}

/** Текущая страница: не ссылка, `aria-current="page"`. */
export function BreadcrumbPage({ className, ...props }: React.ComponentProps<"span">) {
  return (
    <span
      data-slot="breadcrumb-page"
      aria-current="page"
      className={cx("text-foreground", className)}
      {...props}
    />
  );
}

export function BreadcrumbSeparator({ children, className, ...props }: React.ComponentProps<"li">) {
  return (
    <li
      data-slot="breadcrumb-separator"
      role="presentation"
      aria-hidden="true"
      className={cx("text-hint [&>svg]:size-4", className)}
      {...props}
    >
      {children ?? <ChevronRightIcon />}
    </li>
  );
}
