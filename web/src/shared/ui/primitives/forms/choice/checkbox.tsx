import { Checkbox as CheckboxPrimitive } from "@base-ui/react/checkbox";
import { CheckIcon, MinusIcon } from "lucide-react";
import { cx, type WithClassName } from "~/shared/lib/core";

// after:-inset расширяет зону нажатия до ≥ 24 px (WCAG 2.5.8).
export function Checkbox({
  className,
  indeterminate,
  ...props
}: WithClassName<CheckboxPrimitive.Root.Props>) {
  return (
    <CheckboxPrimitive.Root
      data-slot="checkbox"
      className={cx(
        "peer relative flex size-4 shrink-0 items-center justify-center rounded-xs border border-input bg-card transition-colors duration-(--dur-fast) after:absolute after:-inset-x-3 after:-inset-y-2 data-checked:border-primary data-checked:bg-primary data-checked:text-primary-foreground data-indeterminate:border-primary data-indeterminate:bg-primary data-indeterminate:text-primary-foreground data-disabled:cursor-not-allowed data-disabled:border-border data-disabled:bg-surface-high data-disabled:text-muted-foreground aria-invalid:border-error-strong",
        className,
      )}
      indeterminate={indeterminate}
      {...props}
    >
      <CheckboxPrimitive.Indicator
        data-slot="checkbox-indicator"
        className="grid place-content-center text-current transition-none [&>svg]:size-3.5"
      >
        {indeterminate ? <MinusIcon /> : <CheckIcon />}
      </CheckboxPrimitive.Indicator>
    </CheckboxPrimitive.Root>
  );
}
