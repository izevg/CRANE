import { Select as SelectPrimitive } from "@base-ui/react/select";
import { CheckIcon, ChevronDownIcon, ChevronUpIcon } from "lucide-react";
import { cx, type WithClassName } from "~/shared/lib/core";

type SelectSize = "sm" | "default";

const SIZE = {
  default: "h-9",
  sm: "h-8",
} satisfies Record<SelectSize, string>;

/** Корень: управляет значением и открытием. Передавайте `items` — по ним строится подпись значения. */
export function Select<Value, Multiple extends boolean | undefined = false>(
  props: SelectPrimitive.Root.Props<Value, Multiple>,
) {
  return <SelectPrimitive.Root {...props} />;
}

export function SelectGroup({ className, ...props }: WithClassName<SelectPrimitive.Group.Props>) {
  return (
    <SelectPrimitive.Group
      data-slot="select-group"
      className={cx("scroll-my-1 p-1", className)}
      {...props}
    />
  );
}

export function SelectLabel({
  className,
  ...props
}: WithClassName<SelectPrimitive.GroupLabel.Props>) {
  return (
    <SelectPrimitive.GroupLabel
      data-slot="select-label"
      className={cx("px-2 py-1 text-caption text-muted-foreground", className)}
      {...props}
    />
  );
}

export function SelectValue({ className, ...props }: WithClassName<SelectPrimitive.Value.Props>) {
  return (
    <SelectPrimitive.Value
      data-slot="select-value"
      className={cx("flex flex-1 text-left", className)}
      {...props}
    />
  );
}

type TriggerProps = WithClassName<SelectPrimitive.Trigger.Props> & { size?: SelectSize };

export function SelectTrigger({ className, size = "default", children, ...props }: TriggerProps) {
  return (
    <SelectPrimitive.Trigger
      data-slot="select-trigger"
      className={cx(
        "flex w-fit items-center justify-between gap-2 rounded-md border border-input bg-card py-2 pr-2 pl-3 text-body whitespace-nowrap transition-colors duration-(--dur-fast) select-none hover:bg-accent disabled:cursor-not-allowed disabled:bg-surface-high disabled:text-muted-foreground aria-invalid:border-error-strong data-placeholder:text-hint *:data-[slot=select-value]:line-clamp-1 *:data-[slot=select-value]:flex *:data-[slot=select-value]:items-center *:data-[slot=select-value]:gap-2 [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        SIZE[size],
        className,
      )}
      {...props}
    >
      {children}
      <SelectPrimitive.Icon
        render={<ChevronDownIcon className="pointer-events-none size-4 text-muted-foreground" />}
      />
    </SelectPrimitive.Trigger>
  );
}

const SCROLL_ARROW =
  "z-10 flex w-full cursor-default items-center justify-center bg-popover py-1 [&_svg:not([class*='size-'])]:size-4";

export function SelectContent({
  className,
  children,
  side = "bottom",
  sideOffset = 4,
  align = "center",
  ...props
}: WithClassName<SelectPrimitive.Popup.Props> &
  Pick<SelectPrimitive.Positioner.Props, "align" | "side" | "sideOffset">) {
  return (
    <SelectPrimitive.Portal>
      <SelectPrimitive.Positioner
        side={side}
        sideOffset={sideOffset}
        align={align}
        alignItemWithTrigger={false}
        className="isolate z-50"
      >
        <SelectPrimitive.Popup
          data-slot="select-content"
          className={cx(
            "anim-popup relative isolate z-50 max-h-(--available-height) min-w-(--anchor-width) overflow-x-hidden overflow-y-auto rounded-md bg-popover text-popover-foreground shadow-card",
            className,
          )}
          {...props}
        >
          <SelectPrimitive.ScrollUpArrow className={cx(SCROLL_ARROW, "top-0")}>
            <ChevronUpIcon />
          </SelectPrimitive.ScrollUpArrow>
          <SelectPrimitive.List>{children}</SelectPrimitive.List>
          <SelectPrimitive.ScrollDownArrow className={cx(SCROLL_ARROW, "bottom-0")}>
            <ChevronDownIcon />
          </SelectPrimitive.ScrollDownArrow>
        </SelectPrimitive.Popup>
      </SelectPrimitive.Positioner>
    </SelectPrimitive.Portal>
  );
}

export function SelectItem({
  className,
  children,
  ...props
}: WithClassName<SelectPrimitive.Item.Props>) {
  return (
    <SelectPrimitive.Item
      data-slot="select-item"
      className={cx(
        "relative flex h-9 w-full cursor-default items-center gap-2 rounded-sm pr-8 pl-2 text-body select-none data-highlighted:bg-accent data-disabled:cursor-not-allowed data-disabled:text-muted-foreground [&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        className,
      )}
      {...props}
    >
      <SelectPrimitive.ItemText className="flex flex-1 shrink-0 gap-2 whitespace-nowrap">
        {children}
      </SelectPrimitive.ItemText>
      <SelectPrimitive.ItemIndicator
        render={
          <span className="pointer-events-none absolute right-2 flex size-4 items-center justify-center" />
        }
      >
        <CheckIcon className="pointer-events-none" />
      </SelectPrimitive.ItemIndicator>
    </SelectPrimitive.Item>
  );
}
