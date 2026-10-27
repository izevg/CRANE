import { Popover as PopoverPrimitive } from "@base-ui/react/popover";
import { cx, type WithClassName } from "~/shared/lib/core";

type PopoverSize = "default" | "wide";

const SIZE = {
  default: "w-72",
  wide: "w-[360px]",
} satisfies Record<PopoverSize, string>;

/** Без отступов и gap: содержимое раскладывается само. */
const FLUSH = "gap-0 p-0";
const PADDED = "gap-2.5 p-3";

export function Popover(props: PopoverPrimitive.Root.Props) {
  return <PopoverPrimitive.Root {...props} />;
}

export function PopoverTrigger(props: PopoverPrimitive.Trigger.Props) {
  return <PopoverPrimitive.Trigger data-slot="popover-trigger" {...props} />;
}

export function PopoverContent({
  className,
  align = "center",
  side = "bottom",
  sideOffset = 4,
  size = "default",
  flush = false,
  ...props
}: WithClassName<PopoverPrimitive.Popup.Props> &
  Pick<PopoverPrimitive.Positioner.Props, "align" | "side" | "sideOffset"> & {
    /** `wide` — 360 px (центр уведомлений). */
    size?: PopoverSize;
    flush?: boolean;
  }) {
  return (
    <PopoverPrimitive.Portal>
      <PopoverPrimitive.Positioner
        align={align}
        side={side}
        sideOffset={sideOffset}
        className="isolate z-50"
      >
        <PopoverPrimitive.Popup
          data-slot="popover-content"
          className={cx(
            "anim-popup z-50 flex flex-col rounded-md bg-popover [--timeline-surface:var(--popover)] text-body text-popover-foreground shadow-card",
            SIZE[size],
            flush ? FLUSH : PADDED,
            className,
          )}
          {...props}
        />
      </PopoverPrimitive.Positioner>
    </PopoverPrimitive.Portal>
  );
}

export function PopoverTitle({ className, ...props }: WithClassName<PopoverPrimitive.Title.Props>) {
  return (
    <PopoverPrimitive.Title
      data-slot="popover-title"
      className={cx("font-semibold", className)}
      {...props}
    />
  );
}

export function PopoverDescription({
  className,
  ...props
}: WithClassName<PopoverPrimitive.Description.Props>) {
  return (
    <PopoverPrimitive.Description
      data-slot="popover-description"
      className={cx("text-muted-foreground", className)}
      {...props}
    />
  );
}
