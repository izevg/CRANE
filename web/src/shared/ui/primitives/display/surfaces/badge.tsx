import { mergeProps } from "@base-ui/react/merge-props";
import { useRender } from "@base-ui/react/use-render";
import { cx } from "~/shared/lib/core";
import { TONE_SURFACE } from "../tone/tone";

type BadgeVariant =
  | "default"
  | "secondary"
  | "outline"
  | "destructive"
  | "inherit"
  | "inverse"
  | "control";

const VARIANT = {
  // цвет рамки задаёт сам вариант: общий border-transparent в базе конфликтовал бы с border-* варианта
  default: "border-transparent bg-primary text-primary-foreground",
  secondary: "border-transparent bg-secondary text-secondary-foreground",
  outline: "border-border text-foreground",
  destructive: cx("border-transparent", TONE_SURFACE.error),
  // цвет и рамка берутся у родителя: метка на заливке кнопки (primary, destructive) остаётся читаемой
  inherit: "border-current text-current",
  // метка на заливке primary-кнопки: инверсная поверхность (белая с цветом бренда), контраст к заливке ≥ 3:1
  inverse: "border-transparent bg-primary-foreground text-primary",
  // метка на светлой/прозрачной кнопке: своя поверхность и рамка контрола (--input даёт ≥ 3:1, WCAG 1.4.11)
  control: "border-input bg-card text-foreground",
} satisfies Record<BadgeVariant, string>;

type BadgeProps = useRender.ComponentProps<"span"> & { variant?: BadgeVariant };

/** Нейтральная метка (формат, флаг, «MFA»). Статус с тоном — это `StatusBadge`. */
export function Badge({ className, variant = "default", render, ...props }: BadgeProps) {
  return useRender({
    defaultTagName: "span",
    props: mergeProps<"span">(
      {
        className: cx(
          "inline-flex h-5 w-fit shrink-0 items-center justify-center gap-1 overflow-hidden rounded-full border px-2 py-0.5 text-caption font-semibold whitespace-nowrap [&>svg]:pointer-events-none [&>svg]:size-3!",
          VARIANT[variant],
          className,
        ),
      },
      props,
    ),
    render,
    state: { slot: "badge", variant },
  });
}
