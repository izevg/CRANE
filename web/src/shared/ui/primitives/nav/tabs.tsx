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

const H = "group-data-[orientation=horizontal]/tabs:";
const V = "group-data-[orientation=vertical]/tabs:";
// Типизированное деление длин (`calc(<length> / 1px)`) нужно, чтобы получить масштаб из `--active-tab-width`.
const SCALE_OK = "supports-[scale:calc(10px/1px)]:";

/**
 * Индикатор — полоска 1 px, которую двигают `translate` и растягивают `scale` (origin у края вкладок): Layout не
 * запускается (spec 2). Позицию и размер вкладки даёт Base UI через `--active-tab-*`. Без поддержки деления длин
 * в CSS размер меняется скачком, а скользит только `translate`. Токены движения короче при reduced motion (motion.css).
 */
const INDICATOR = [
  "pointer-events-none absolute bg-brand transition-[translate,scale] duration-(--dur-base) ease-(--ease-out) motion-reduce:duration-0 in-data-[motion=reduce]:duration-0 forced-colors:bg-[Highlight]",
  `${H}bottom-0 ${H}left-0 ${H}h-0.5 ${H}origin-left ${H}w-(--active-tab-width) ${H}translate-x-(--active-tab-left)`,
  `${H}${SCALE_OK}w-px ${H}${SCALE_OK}scale-x-[calc(var(--active-tab-width)/1px)]`,
  `${V}top-0 ${V}right-0 ${V}w-0.5 ${V}origin-top ${V}h-(--active-tab-height) ${V}translate-y-(--active-tab-top)`,
  `${V}${SCALE_OK}h-px ${V}${SCALE_OK}scale-y-[calc(var(--active-tab-height)/1px)]`,
].join(" ");

function TabsIndicator() {
  return (
    <TabsPrimitive.Indicator data-slot="tabs-indicator" aria-hidden="true" className={INDICATOR} />
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
