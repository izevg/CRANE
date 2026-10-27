import { Popover as PopoverPrimitive } from "@base-ui/react/popover";
import { cx, type WithClassName } from "~/shared/lib/core";

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
  ...props
}: WithClassName<PopoverPrimitive.Popup.Props> &
  Pick<PopoverPrimitive.Positioner.Props, "align" | "side" | "sideOffset">) {
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
            "anim-popup z-50 flex w-72 flex-col gap-2.5 rounded-md bg-popover p-3 text-body text-popover-foreground shadow-card",
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
