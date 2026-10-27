// i18n: не мигрирован
import { Dialog as DialogPrimitive } from "@base-ui/react/dialog";
import { XIcon } from "lucide-react";
import type * as React from "react";
import { cx, type WithClassName } from "~/shared/lib/core";
import { Button } from "../../forms/toggle/button";

const CLOSE = "Закрыть";

type DialogSize = "default" | "palette";

const SIZE = {
  default: "sm:max-w-md",
  palette: "sm:max-w-[560px]",
} satisfies Record<DialogSize, string>;

/** Без отступов и gap: содержимое (например, список палитры) раскладывается само. */
const FLUSH = "gap-0 p-0";
const PADDED = "gap-4 p-6";

export function Dialog(props: DialogPrimitive.Root.Props) {
  return <DialogPrimitive.Root {...props} />;
}

export function DialogTrigger(props: DialogPrimitive.Trigger.Props) {
  return <DialogPrimitive.Trigger data-slot="dialog-trigger" {...props} />;
}

export function DialogClose(props: DialogPrimitive.Close.Props) {
  return <DialogPrimitive.Close data-slot="dialog-close" {...props} />;
}

export function DialogContent({
  className,
  children,
  showCloseButton = true,
  size = "default",
  flush = false,
  ...props
}: WithClassName<DialogPrimitive.Popup.Props> & {
  showCloseButton?: boolean;
  /** `palette` — 560 px для командной палитры. */
  size?: DialogSize;
  flush?: boolean;
}) {
  return (
    <DialogPrimitive.Portal>
      <DialogPrimitive.Backdrop className="anim-scrim fixed inset-0 isolate z-50 bg-(--scrim)" />
      <DialogPrimitive.Popup
        data-slot="dialog-content"
        className={cx(
          "anim-dialog fixed top-1/2 left-1/2 z-50 flex max-h-[calc(100dvh-2rem)] w-full max-w-[calc(100%-2rem)] flex-col -translate-x-1/2 -translate-y-1/2 rounded-xl bg-popover text-body text-popover-foreground shadow-card outline-none",
          SIZE[size],
          flush ? FLUSH : PADDED,
          className,
        )}
        {...props}
      >
        {children}
        {showCloseButton ? (
          <DialogPrimitive.Close
            data-slot="dialog-close"
            render={
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={CLOSE}
                className="absolute top-4 right-4"
              />
            }
          >
            <XIcon />
          </DialogPrimitive.Close>
        ) : null}
      </DialogPrimitive.Popup>
    </DialogPrimitive.Portal>
  );
}

export function DialogHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="dialog-header"
      className={cx("flex shrink-0 flex-col gap-2", className)}
      {...props}
    />
  );
}

/** Прокручиваемое тело: заголовок и подвал остаются на виду, длинное содержимое прокручивается внутри (WCAG 1.4.10). */
export function DialogBody({ className, ...props }: React.ComponentProps<"div">) {
  // -m-1/p-1: запас под outline элементов в фокусе, чтобы край области прокрутки его не обрезал
  return (
    <div
      data-slot="dialog-body"
      className={cx("-m-1 min-h-0 flex-1 overflow-y-auto p-1", className)}
      {...props}
    />
  );
}

export function DialogFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="dialog-footer"
      className={cx("flex shrink-0 flex-col-reverse gap-2 sm:flex-row sm:justify-end", className)}
      {...props}
    />
  );
}

export function DialogTitle({ className, ...props }: WithClassName<DialogPrimitive.Title.Props>) {
  return (
    <DialogPrimitive.Title
      data-slot="dialog-title"
      className={cx("text-h3", className)}
      {...props}
    />
  );
}

export function DialogDescription({
  className,
  ...props
}: WithClassName<DialogPrimitive.Description.Props>) {
  return (
    <DialogPrimitive.Description
      data-slot="dialog-description"
      className={cx("text-body text-muted-foreground", className)}
      {...props}
    />
  );
}
