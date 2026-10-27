import { Tabs as TabsPrimitive } from "@base-ui/react/tabs";
import { LazyMotion, m, useReducedMotion } from "motion/react";
import { createContext, useCallback, useContext, useEffect, useId, useRef, useState } from "react";
import { cx, useScrollOverflow, type WithClassName } from "~/shared/lib/core";

type IndicatorContext = { layoutId: string; active: unknown };

/** `layoutId` у каждого экземпляра Tabs свой: индикаторы разных наборов вкладок не «перелетают» друг в друга. */
const IndicatorState = createContext<IndicatorContext>({ layoutId: "", active: undefined });

const INDICATOR_SPRING = { type: "spring", duration: 0.2, bounce: 0 } as const;
const INSTANT = { duration: 0 } as const;

const loadFeatures = () => import("./motion-features").then((module) => module.default);

/** Индикатор скользит между вкладками (Motion `layoutId`); при reduced-motion перемещается мгновенно. */
function TabIndicator({ layoutId, className }: { layoutId: string; className: string }) {
  const reduced = useReducedMotion();
  return (
    <m.span
      aria-hidden="true"
      data-slot="tabs-indicator"
      layoutId={layoutId}
      transition={reduced ? INSTANT : INDICATOR_SPRING}
      className={cx("absolute bg-brand forced-colors:bg-[Highlight]", className)}
    />
  );
}

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
  value,
  defaultValue,
  onValueChange,
  ...props
}: WithClassName<TabsPrimitive.Root.Props>) {
  const layoutId = useId();
  const [inner, setInner] = useState<unknown>(defaultValue);
  const handleValueChange = useCallback<NonNullable<TabsPrimitive.Root.Props["onValueChange"]>>(
    (next, details) => {
      setInner(next);
      onValueChange?.(next, details);
    },
    [onValueChange],
  );
  return (
    <LazyMotion features={loadFeatures} strict>
      <IndicatorState.Provider value={{ layoutId, active: value ?? inner }}>
        <TabsPrimitive.Root
          data-slot="tabs"
          orientation={orientation}
          value={value}
          defaultValue={defaultValue}
          onValueChange={handleValueChange}
          className={cx("group/tabs flex gap-2 data-[orientation=horizontal]:flex-col", className)}
          {...props}
        />
      </IndicatorState.Provider>
    </LazyMotion>
  );
}

type TabsListVariant = "default" | "line";

const LIST_VARIANT = {
  default: "bg-muted",
  line: "scroll-px-8 justify-start gap-1 bg-transparent group-data-[orientation=horizontal]/tabs:overflow-x-auto group-data-[orientation=horizontal]/tabs:overflow-y-hidden",
} satisfies Record<TabsListVariant, string>;

/** Стрелки переключают вкладку сразу (`activateOnFocus`). Вариант `line` на узкой ширине прокручивается по горизонтали, активная вкладка остаётся в видимой области. */
export function TabsList({
  className,
  variant = "default",
  activateOnFocus = true,
  ...props
}: WithClassName<TabsPrimitive.List.Props> & { variant?: TabsListVariant }) {
  const { active } = useContext(IndicatorState);
  const scrollRef = useRef<HTMLDivElement>(null);
  const wrapperRef = useRef<HTMLDivElement>(null);
  useScrollOverflow(scrollRef, wrapperRef);
  // biome-ignore lint/correctness/useExhaustiveDependencies: при смене активной вкладки показываем её в области прокрутки
  useEffect(() => {
    scrollRef.current
      ?.querySelector<HTMLElement>("[data-active]")
      ?.scrollIntoView({ inline: "nearest", block: "nearest" });
  }, [active]);

  const list = (
    <TabsPrimitive.List
      ref={scrollRef}
      data-slot="tabs-list"
      data-variant={variant}
      activateOnFocus={activateOnFocus}
      className={cx(
        "group/tabs-list inline-flex w-fit max-w-full items-center justify-center rounded-md p-[3px] text-muted-foreground group-data-[orientation=horizontal]/tabs:h-9 group-data-[orientation=vertical]/tabs:h-fit group-data-[orientation=vertical]/tabs:flex-col data-[variant=line]:rounded-none",
        LIST_VARIANT[variant],
        className,
      )}
      {...props}
    />
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
  const { layoutId, active } = useContext(IndicatorState);
  return (
    <TabsPrimitive.Tab
      data-slot="tabs-trigger"
      className={cx(
        "relative inline-flex h-[calc(100%-1px)] flex-1 items-center justify-center gap-1.5 rounded-md border border-transparent px-3 py-0.5 text-body font-semibold whitespace-nowrap text-muted-foreground transition-colors duration-(--dur-fast) group-data-[orientation=vertical]/tabs:w-full group-data-[orientation=vertical]/tabs:justify-start hover:text-foreground disabled:cursor-not-allowed disabled:text-muted-foreground group-data-[variant=default]/tabs-list:data-active:shadow-subtle group-data-[variant=line]/tabs-list:data-active:shadow-none group-data-[variant=line]/tabs-list:bg-transparent group-data-[variant=line]/tabs-list:data-active:bg-transparent group-data-[variant=line]/tabs-list:focus-visible:shadow-[inset_0_0_0_2px_var(--ring)] data-active:bg-card data-active:text-foreground [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        className,
      )}
      {...props}
    >
      {children}
      {active === props.value ? (
        <TabIndicator
          layoutId={layoutId}
          className="group-data-[variant=default]/tabs-list:hidden group-data-[orientation=horizontal]/tabs:inset-x-0 group-data-[orientation=horizontal]/tabs:bottom-0 group-data-[orientation=horizontal]/tabs:h-0.5 group-data-[orientation=vertical]/tabs:inset-y-0 group-data-[orientation=vertical]/tabs:-right-1 group-data-[orientation=vertical]/tabs:w-0.5"
        />
      ) : null}
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
