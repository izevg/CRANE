import { Button as ButtonPrimitive } from "@base-ui/react/button";
import { LoaderCircleIcon } from "lucide-react";
import { cx, type WithClassName } from "~/shared/lib/core";
import { type ButtonVariant, buttonStyles, type IconSize, type TextSize } from "./button-styles";

type ButtonProps = WithClassName<ButtonPrimitive.Props> & {
  variant?: ButtonVariant;
  /** Действие выполняется: кнопка недоступна для нажатия, но остаётся в порядке фокуса, показывает индикатор и `aria-busy`. */
  pending?: boolean;
} & ({ size?: TextSize } | { size: IconSize; "aria-label": string });

/** Иконочная кнопка (`size="icon" | "icon-sm"`) требует `aria-label` на уровне типов. */
export function Button({
  className,
  variant = "primary",
  size = "default",
  pending = false,
  children,
  ...props
}: ButtonProps) {
  return (
    <ButtonPrimitive
      data-slot="button"
      className={cx(buttonStyles({ variant, size }), className)}
      {...(pending ? { disabled: true, focusableWhenDisabled: true, "aria-busy": true } : {})}
      {...props}
    >
      {pending ? (
        <LoaderCircleIcon aria-hidden="true" className="motion-safe:animate-spin" />
      ) : null}
      {children}
    </ButtonPrimitive>
  );
}
