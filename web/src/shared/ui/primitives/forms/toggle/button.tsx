import { Button as ButtonPrimitive } from "@base-ui/react/button";
import { cx, type WithClassName } from "~/shared/lib/core";
import { type ButtonVariant, buttonStyles, type IconSize, type TextSize } from "./button-styles";

type ButtonProps = WithClassName<ButtonPrimitive.Props> & { variant?: ButtonVariant } & (
    | { size?: TextSize }
    | { size: IconSize; "aria-label": string }
  );

/** Иконочная кнопка (`size="icon" | "icon-sm"`) требует `aria-label` на уровне типов. */
export function Button({
  className,
  variant = "primary",
  size = "default",
  ...props
}: ButtonProps) {
  return (
    <ButtonPrimitive
      data-slot="button"
      className={cx(buttonStyles({ variant, size }), className)}
      {...props}
    />
  );
}
