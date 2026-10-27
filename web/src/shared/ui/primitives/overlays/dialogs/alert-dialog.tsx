import { AlertDialog as AlertDialogPrimitive } from "@base-ui/react/alert-dialog";
import type * as React from "react";
import { cx, type WithClassName } from "~/shared/lib/core";
import { Button } from "../../forms/toggle/button";
import type { ButtonVariant } from "../../forms/toggle/button-styles";

export function AlertDialog(props: AlertDialogPrimitive.Root.Props) {
  return <AlertDialogPrimitive.Root {...props} />;
}

export function AlertDialogTrigger(props: AlertDialogPrimitive.Trigger.Props) {
  return <AlertDialogPrimitive.Trigger data-slot="alert-dialog-trigger" {...props} />;
}

type AlertDialogSize = "default" | "wide";

// Классы записаны целиком: Tailwind находит их по тексту исходников. 384 и 448 px (composition §1).
const SIZE = {
  default: "sm:max-w-sm",
  wide: "sm:max-w-md",
} satisfies Record<AlertDialogSize, string>;

/**
 * Не закрывается кликом вне и по умолчанию фокусирует первый фокусируемый элемент.
 * Начальный фокус задаёт `initialFocus` (например, ref на «Отмена» для деструктивного подтверждения).
 */
export function AlertDialogContent({
  className,
  size = "default",
  ...props
}: WithClassName<AlertDialogPrimitive.Popup.Props> & { size?: AlertDialogSize }) {
  return (
    <AlertDialogPrimitive.Portal>
      <AlertDialogPrimitive.Backdrop className="anim-scrim fixed inset-0 isolate z-50 bg-(--scrim)" />
      <AlertDialogPrimitive.Popup
        data-slot="alert-dialog-content"
        className={cx(
          "anim-dialog fixed top-1/2 left-1/2 z-50 flex max-h-[calc(100dvh-2rem)] min-h-0 w-full max-w-[calc(100%-2rem)] flex-col -translate-x-1/2 -translate-y-1/2 gap-4 rounded-xl bg-popover p-6 text-body text-popover-foreground shadow-card outline-none",
          SIZE[size],
          className,
        )}
        {...props}
      />
    </AlertDialogPrimitive.Portal>
  );
}

export function AlertDialogHeader({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="alert-dialog-header"
      className={cx("flex shrink-0 flex-col gap-1.5", className)}
      {...props}
    />
  );
}

/** Прокручиваемое тело: заголовок и подвал остаются на виду, длинное содержимое прокручивается внутри (WCAG 1.4.10). */
export function AlertDialogBody({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="alert-dialog-body"
      className={cx("-m-1 min-h-0 flex-1 overflow-y-auto p-1", className)}
      {...props}
    />
  );
}

export function AlertDialogFooter({ className, ...props }: React.ComponentProps<"div">) {
  return (
    <div
      data-slot="alert-dialog-footer"
      className={cx("flex shrink-0 flex-col-reverse gap-2 sm:flex-row sm:justify-end", className)}
      {...props}
    />
  );
}

export function AlertDialogTitle({
  className,
  ...props
}: WithClassName<AlertDialogPrimitive.Title.Props>) {
  return (
    <AlertDialogPrimitive.Title
      data-slot="alert-dialog-title"
      className={cx("text-h3", className)}
      {...props}
    />
  );
}

export function AlertDialogDescription({
  className,
  ...props
}: WithClassName<AlertDialogPrimitive.Description.Props>) {
  return (
    <AlertDialogPrimitive.Description
      data-slot="alert-dialog-description"
      className={cx("min-h-0 overflow-y-auto text-body text-muted-foreground", className)}
      {...props}
    />
  );
}

/** Закрывает диалог и подтверждает действие: обработчик подтверждения передаётся в `onClick`. */
export function AlertDialogAction({
  variant = "primary",
  ...props
}: WithClassName<AlertDialogPrimitive.Close.Props> & { variant?: ButtonVariant }) {
  return (
    <AlertDialogPrimitive.Close
      data-slot="alert-dialog-action"
      render={<Button variant={variant} />}
      {...props}
    />
  );
}

export function AlertDialogCancel(props: WithClassName<AlertDialogPrimitive.Close.Props>) {
  return (
    <AlertDialogPrimitive.Close
      data-slot="alert-dialog-cancel"
      render={<Button variant="secondary" />}
      {...props}
    />
  );
}
