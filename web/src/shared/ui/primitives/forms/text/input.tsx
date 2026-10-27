import { Input as InputPrimitive } from "@base-ui/react/input";
import { cx, type WithClassName } from "~/shared/lib/core";

/** Внутри `Field` связывается с подписью и ошибкой автоматически (`aria-labelledby`, `aria-describedby`). */
export function Input({ className, ...props }: WithClassName<InputPrimitive.Props>) {
  return (
    <InputPrimitive
      data-slot="input"
      className={cx(
        "h-9 w-full min-w-0 rounded-md border border-input bg-card px-3 text-body text-foreground transition-colors duration-(--dur-fast) file:inline-flex file:h-6 file:border-0 file:bg-transparent file:font-semibold file:text-foreground placeholder:text-hint disabled:cursor-not-allowed disabled:bg-surface-high disabled:text-muted-foreground disabled:placeholder:text-muted-foreground aria-invalid:border-error-strong data-invalid:border-error-strong",
        className,
      )}
      {...props}
    />
  );
}
