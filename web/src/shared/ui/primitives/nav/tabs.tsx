import { Tabs as TabsPrimitive } from "@base-ui/react/tabs";
import { useRef } from "react";
import { cx, useScrollOverflow, type WithClassName } from "~/shared/lib/core";

const FADE =
  "pointer-events-none absolute inset-y-0 z-[1] w-8 from-background to-transparent opacity-0 transition-opacity duration-(--dur-fast) ease-(--ease-out) motion-reduce:transition-none";

/** Затухания по краям полосы вкладок: включаются, когда за краем есть скрытые вкладки (WCAG 1.4.10). */
function TabScrollFades() {
  return (
    <>
      <span
        aria-hidden="true"
        className={cx(
          FADE,
          "left-0 bg-linear-to-r group-data-[overflow-start=true]/tabscroll:opacity-100",
        )}
      />
      <span
        aria-hidden="true"
        className={cx(
          FADE,
          "right-0 bg-linear-to-l group-data-[overflow-end=true]/tabscroll:opacity-100",
        )}
      />
    </>
  );
}

export function Tabs({
  className,
  orientation = "horizontal",
  ...props
}: WithClassName<TabsPrimitive.Root.Props>) {
  return (
    <TabsPrimitive.Root
      data-slot="tabs"
      orientation={orientation}
      className={cx("group/tabs flex gap-2 data-[orientation=horizontal]:flex-col", className)}
      {...props}
    />
  );
}

/**
 * Полоса под активной вкладкой `line`: позицию и размер даёт Base UI через `--active-tab-*`,
 * скольжение — transition на токенах движения (при reduced motion токены короче, см. motion.css).
 */
function TabsIndicator() {
  return (
    <TabsPrimitive.Indicator
      data-slot="tabs-indicator"
      aria-hidden="true"
      className="pointer-events-none absolute bg-brand transition-[translate,width,height] duration-(--dur-base) ease-(--ease-out) forced-colors:bg-[Highlight] group-data-[orientation=horizontal]/tabs:bottom-0 group-data-[orientation=horizontal]/tabs:left-0 group-data-[orientation=horizontal]/tabs:h-0.5 group-data-[orientation=horizontal]/tabs:w-(--active-tab-width) group-data-[orientation=horizontal]/tabs:translate-x-(--active-tab-left) group-data-[orientation=vertical]/tabs:top-0 group-data-[orientation=vertical]/tabs:right-0 group-data-[orientation=vertical]/tabs:h-(--active-tab-height) group-data-[orientation=vertical]/tabs:w-0.5 group-data-[orientation=vertical]/tabs:translate-y-(--active-tab-top)"
    />
  );
}

type TabsListVariant = "default" | "line";

const LIST_VARIANT = {
  default: "bg-muted",
  line: "scroll-px-8 justify-start gap-1 bg-transparent group-data-[orientation=horizontal]/tabs:overflow-x-auto group-data-[orientation=horizontal]/tabs:overflow-y-hidden",
} satisfies Record<TabsListVariant, string>;

/** Стрелки переключают вкладку сразу (`activateOnFocus`). Вариант `line` на узкой ширине прокручивается по горизонтали (WCAG 1.4.10). */
export function TabsList({
  className,
  variant = "default",
  activateOnFocus = true,
  children,
  ...props
}: WithClassName<TabsPrimitive.List.Props> & { variant?: TabsListVariant }) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const wrapperRef = useRef<HTMLDivElement>(null);
  useScrollOverflow(scrollRef, wrapperRef);
  const list = (
    <TabsPrimitive.List
      ref={scrollRef}
      data-slot="tabs-list"
      data-variant={variant}
      activateOnFocus={activateOnFocus}
      className={cx(
        "group/tabs-list relative inline-flex w-fit max-w-full items-center justify-center rounded-md p-[3px] text-muted-foreground group-data-[orientation=horizontal]/tabs:h-9 group-data-[orientation=vertical]/tabs:h-fit group-data-[orientation=vertical]/tabs:flex-col data-[variant=line]:rounded-none",
        LIST_VARIANT[variant],
        className,
      )}
      {...props}
    >
      {children}
      {variant === "line" ? <TabsIndicator /> : null}
    </TabsPrimitive.List>
  );
  if (variant !== "line") {
    return list;
  }
  return (
    <div ref={wrapperRef} data-slot="tabs-scroll" className="group/tabscroll relative min-w-0">
      {list}
      <TabScrollFades />
    </div>
  );
}

export function TabsTrigger({
  className,
  children,
  ...props
}: WithClassName<TabsPrimitive.Tab.Props>) {
  return (
    <TabsPrimitive.Tab
      data-slot="tabs-trigger"
      className={cx(
        "relative inline-flex h-[calc(100%-1px)] flex-1 items-center justify-center gap-1.5 rounded-md border border-transparent px-3 py-0.5 text-body font-semibold whitespace-nowrap text-muted-foreground transition-colors duration-(--dur-fast) group-data-[orientation=vertical]/tabs:w-full group-data-[orientation=vertical]/tabs:justify-start hover:text-foreground disabled:cursor-not-allowed disabled:text-muted-foreground group-data-[variant=default]/tabs-list:data-active:shadow-subtle group-data-[variant=line]/tabs-list:data-active:shadow-none group-data-[variant=line]/tabs-list:bg-transparent group-data-[variant=line]/tabs-list:data-active:bg-transparent group-data-[variant=line]/tabs-list:focus-visible:-outline-offset-2 data-active:bg-card data-active:text-foreground [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        className,
      )}
      {...props}
    >
      {children}
    </TabsPrimitive.Tab>
  );
}

export function TabsContent({ className, ...props }: WithClassName<TabsPrimitive.Panel.Props>) {
  return (
    <TabsPrimitive.Panel
      data-slot="tabs-content"
      className={cx("flex-1 text-body", className)}
      {...props}
    />
  );
}
