import { Tooltip as TooltipPrimitive } from "@base-ui/react/tooltip";
import { cx, type WithClassName } from "~/shared/lib/core";

const OPEN_DELAY_MS = 400;
const GROUP_TIMEOUT_MS = 300;
const SIDE_OFFSET_PX = 6;

/** Общая задержка показа для всех подсказок поддерева: монтируется один раз в корне. */
export function TooltipProvider({
  delay = OPEN_DELAY_MS,
  closeDelay = 0,
  timeout = GROUP_TIMEOUT_MS,
  ...props
}: TooltipPrimitive.Provider.Props) {
  return (
    <TooltipPrimitive.Provider delay={delay} closeDelay={closeDelay} timeout={timeout} {...props} />
  );
}

export function Tooltip(props: TooltipPrimitive.Root.Props) {
  return <TooltipPrimitive.Root {...props} />;
}

export function TooltipTrigger(props: TooltipPrimitive.Trigger.Props) {
  return <TooltipPrimitive.Trigger data-slot="tooltip-trigger" {...props} />;
}

export function TooltipContent({
  className,
  side = "top",
  sideOffset = SIDE_OFFSET_PX,
  align = "center",
  children,
  ...props
}: WithClassName<TooltipPrimitive.Popup.Props> &
  Pick<TooltipPrimitive.Positioner.Props, "align" | "side" | "sideOffset">) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Positioner
        align={align}
        side={side}
        sideOffset={sideOffset}
        className="isolate z-50"
      >
        <TooltipPrimitive.Popup
          data-slot="tooltip-content"
          className={cx(
            "anim-fade z-50 inline-flex w-fit max-w-xs items-center gap-1.5 rounded-sm bg-inverse px-3 py-1.5 text-caption text-inverse-foreground has-data-[slot=kbd]:pr-1.5 **:data-[slot=kbd]:border-transparent **:data-[slot=kbd]:bg-inverse-foreground/15 **:data-[slot=kbd]:text-inverse-foreground",
            className,
          )}
          {...props}
        >
          {children}
        </TooltipPrimitive.Popup>
      </TooltipPrimitive.Positioner>
    </TooltipPrimitive.Portal>
  );
}
