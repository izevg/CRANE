// i18n: не мигрирован
import { Dialog as SheetPrimitive } from "@base-ui/react/dialog";
import { XIcon } from "lucide-react";
import type * as React from "react";
import { cx, type WithClassName } from "~/shared/lib/core";
import { Button } from "../../forms/toggle/button";

const CLOSE = "Закрыть";

type SheetSide = "top" | "right" | "bottom" | "left";

const SIDE = {
  top: "inset-x-0 top-0 h-auto border-b",
  right: "inset-y-0 right-0 h-full w-3/4 border-l sm:max-w-sm",
  bottom: "inset-x-0 bottom-0 h-auto border-t",
  left: "inset-y-0 left-0 h-full w-3/4 border-r sm:max-w-sm",
} satisfies Record<SheetSide, string>;

export function Sheet(props: SheetPrimitive.Root.Props) {
  return <SheetPrimitive.Root {...props} />;
}

export function SheetTrigger(props: SheetPrimitive.Trigger.Props) {
  return <SheetPrimitive.Trigger data-slot="sheet-trigger" {...props} />;
}

export function SheetContent({
  className,
  children,
  side = "right",
  showCloseButton = true,
  ...props
}: WithClassName<SheetPrimitive.Popup.Props> & { side?: SheetSide; showCloseButton?: boolean }) {
  return (
    <SheetPrimitive.Portal>
      <SheetPrimitive.Backdrop className="anim-scrim fixed inset-0 z-50 bg-(--scrim)" />
      <SheetPrimitive.Popup
        data-slot="sheet-content"
        data-side={side}
        className={cx(
          "anim-sheet fixed z-50 flex max-h-dvh flex-col gap-4 bg-popover text-body text-popover-foreground shadow-card outline-none",
          SIDE[side],
          className,
        )}
        {...props}
      >
        {children}
        {showCloseButton ? (
          <SheetPrimitive.Close
            data-slot="sheet-close"
            render={
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={CLOSE}
                className="absolute top-3 right-3"
              />
            }
          >
            <XIcon />
          </SheetPrimitive.Close>
        ) : null}
      </SheetPrimitive.Popup>
    </SheetPrimitive.Portal>
  );
}

export function SheetHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sheet-header"
      className={cx("flex shrink-0 flex-col gap-0.5 p-4", className)}
      {...props}
    />
  );
}

/** Прокручиваемое тело: заголовок и подвал остаются на виду, длинное содержимое прокручивается внутри (WCAG 1.4.10). */
export function SheetBody({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sheet-body"
      className={cx("min-h-0 flex-1 overflow-y-auto px-4", className)}
      {...props}
    />
  );
}

export function SheetFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="sheet-footer"
      className={cx("mt-auto flex shrink-0 flex-col gap-2 p-4", className)}
      {...props}
    />
  );
}

export function SheetTitle({ className, ...props }: WithClassName<SheetPrimitive.Title.Props>) {
  return (
    <SheetPrimitive.Title
      data-slot="sheet-title"
      className={cx("text-h3 text-foreground", className)}
      {...props}
    />
  );
}

export function SheetDescription({
  className,
  ...props
}: WithClassName<SheetPrimitive.Description.Props>) {
  return (
    <SheetPrimitive.Description
      data-slot="sheet-description"
      className={cx("text-body text-muted-foreground", className)}
      {...props}
    />
  );
}
