import { Toggle as TogglePrimitive } from "@base-ui/react/toggle";
import { ToggleGroup as ToggleGroupPrimitive } from "@base-ui/react/toggle-group";
import { createContext, useContext } from "react";
import { cx, type WithClassName } from "~/shared/lib/core";

type ToggleVariant = "default" | "outline";
type ToggleSize = "sm" | "default";

const VARIANT = {
  default: "bg-transparent",
  outline: "border border-input bg-transparent hover:bg-accent data-pressed:border-brand",
} satisfies Record<ToggleVariant, string>;

const SIZE = {
  default: "h-9 min-w-9 px-3",
  sm: "h-8 min-w-8 px-3",
} satisfies Record<ToggleSize, string>;

type GroupStyle = { variant: ToggleVariant; size: ToggleSize };

const GroupStyleContext = createContext<GroupStyle>({ variant: "default", size: "default" });

type ToggleGroupProps = WithClassName<ToggleGroupPrimitive.Props> & Partial<GroupStyle>;

/** Группа переключателей (основа SegmentedControl). Вид задаётся один раз на группу. */
export function ToggleGroup({
  className,
  variant = "default",
  size = "default",
  ...props
}: ToggleGroupProps) {
  return (
    <GroupStyleContext.Provider value={{ variant, size }}>
      <ToggleGroupPrimitive
        data-slot="toggle-group"
        className={cx("flex w-fit flex-row items-center gap-1 rounded-md", className)}
        {...props}
      />
    </GroupStyleContext.Provider>
  );
}

export function ToggleGroupItem({ className, ...props }: WithClassName<TogglePrimitive.Props>) {
  const { variant, size } = useContext(GroupStyleContext);
  return (
    <TogglePrimitive
      data-slot="toggle-group-item"
      className={cx(
        "inline-flex shrink-0 items-center justify-center gap-1 rounded-md text-body font-semibold whitespace-nowrap transition-colors duration-(--dur-fast) ease-(--ease-out) hover:bg-accent hover:text-foreground focus-visible:z-10 data-disabled:cursor-not-allowed data-disabled:text-muted-foreground data-pressed:relative data-pressed:z-10 data-pressed:bg-brand-container data-pressed:text-brand data-pressed:inset-ring data-pressed:inset-ring-brand data-pressed:hover:bg-brand-container [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        VARIANT[variant],
        SIZE[size],
        className,
      )}
      {...props}
    />
  );
}
