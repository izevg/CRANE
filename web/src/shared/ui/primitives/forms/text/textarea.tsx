import { Field as FieldPrimitive } from "@base-ui/react/field";
import { cx, type WithClassName } from "~/shared/lib/core";

/** `Field.Control` в виде `<textarea>`: внутри `Field` связывается с подписью и ошибкой, как `Input`. */
export function Textarea({ className, ...props }: WithClassName<FieldPrimitive.Control.Props>) {
  return (
    <FieldPrimitive.Control
      data-slot="textarea"
      render={<textarea />}
      className={cx(
        "flex field-sizing-content min-h-16 w-full rounded-md border border-input bg-card px-3 py-2 text-body text-foreground transition-colors duration-(--dur-fast) placeholder:text-hint disabled:cursor-not-allowed disabled:bg-surface-high disabled:text-muted-foreground disabled:placeholder:text-muted-foreground aria-invalid:border-error-strong data-invalid:border-error-strong",
        className,
      )}
      {...props}
    />
  );
}
