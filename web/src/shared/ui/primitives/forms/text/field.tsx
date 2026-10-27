import { Field as FieldPrimitive } from "@base-ui/react/field";
import { cx, type WithClassName } from "~/shared/lib/core";

/** Поле формы: подпись, контрол, подсказка и ошибка связаны ARIA без ручных id. */
export function Field({ className, ...props }: WithClassName<FieldPrimitive.Root.Props>) {
  return (
    <FieldPrimitive.Root
      data-slot="field"
      className={cx("flex flex-col gap-1.5", className)}
      {...props}
    />
  );
}

export function FieldLabel({ className, ...props }: WithClassName<FieldPrimitive.Label.Props>) {
  return (
    <FieldPrimitive.Label
      data-slot="field-label"
      className={cx(
        "flex select-none items-center gap-2 text-body font-semibold data-disabled:text-muted-foreground",
        className,
      )}
      {...props}
    />
  );
}

export function FieldDescription({
  className,
  ...props
}: WithClassName<FieldPrimitive.Description.Props>) {
  return (
    <FieldPrimitive.Description
      data-slot="field-description"
      className={cx("text-caption text-muted-foreground", className)}
      {...props}
    />
  );
}

/** Показывается при невалидном значении; `match` задаёт условие (по умолчанию — по `ValidityState`). */
export function FieldError({ className, ...props }: WithClassName<FieldPrimitive.Error.Props>) {
  return (
    <FieldPrimitive.Error
      data-slot="field-error"
      className={cx("text-caption text-error", className)}
      {...props}
    />
  );
}
