import type * as React from "react";
import { useLayoutEffect, useRef } from "react";
import { cx } from "~/shared/lib/core";

/**
 * Контейнер прокручивается только при переполнении, и только тогда он попадает в Tab (WCAG 2.1.1, axe
 * `scrollable-region-focusable`): на широком экране лишней остановки нет.
 */
export function Table({ className, ...props }: React.ComponentProps<"table">) {
  const ref = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    const container = ref.current;
    if (!container) {
      return;
    }
    const sync = () => {
      if (container.scrollWidth > container.clientWidth) {
        container.setAttribute("tabindex", "0");
      } else {
        container.removeAttribute("tabindex");
      }
    };
    sync();
    const observer = new ResizeObserver(sync);
    observer.observe(container);
    const table = container.firstElementChild;
    if (table) {
      observer.observe(table);
    }
    return () => observer.disconnect();
  }, []);
  return (
    <div ref={ref} data-slot="table-container" className="relative w-full overflow-x-auto">
      <table
        data-slot="table"
        className={cx("w-full caption-bottom text-body", className)}
        {...props}
      />
    </div>
  );
}

export function TableHeader({ className, ...props }: React.ComponentProps<"thead">) {
  return <thead data-slot="table-header" className={cx("[&_tr]:border-b", className)} {...props} />;
}

export function TableBody({ className, ...props }: React.ComponentProps<"tbody">) {
  return (
    <tbody
      data-slot="table-body"
      className={cx("[&_tr:last-child]:border-0", className)}
      {...props}
    />
  );
}

export function TableRow({ className, ...props }: React.ComponentProps<"tr">) {
  return (
    <tr
      data-slot="table-row"
      className={cx(
        "border-b transition-colors duration-(--dur-instant) hover:bg-accent has-aria-expanded:bg-accent data-[state=selected]:bg-brand-container",
        className,
      )}
      {...props}
    />
  );
}

export function TableHead({ className, ...props }: React.ComponentProps<"th">) {
  return (
    <th
      data-slot="table-head"
      scope="col"
      className={cx(
        "h-10 px-2 text-left align-middle text-overline whitespace-nowrap text-muted-foreground [&:has([role=checkbox])]:pr-0",
        className,
      )}
      {...props}
    />
  );
}

export function TableCell({ className, ...props }: React.ComponentProps<"td">) {
  return (
    <td
      data-slot="table-cell"
      className={cx("p-2 align-middle whitespace-nowrap [&:has([role=checkbox])]:pr-0", className)}
      {...props}
    />
  );
}

export function TableCaption({ className, ...props }: React.ComponentProps<"caption">) {
  return (
    <caption
      data-slot="table-caption"
      className={cx("mt-4 text-body text-muted-foreground", className)}
      {...props}
    />
  );
}
