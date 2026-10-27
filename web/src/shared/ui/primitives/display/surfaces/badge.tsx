import { mergeProps } from "@base-ui/react/merge-props";
import { useRender } from "@base-ui/react/use-render";
import { cx } from "~/shared/lib/core";
import { TONE_SURFACE } from "../tone/tone";

type BadgeVariant = "default" | "secondary" | "outline" | "destructive";

const VARIANT = {
  default: "bg-primary text-primary-foreground",
  secondary: "bg-secondary text-secondary-foreground",
  outline: "border-border text-foreground",
  destructive: TONE_SURFACE.error,
} satisfies Record<BadgeVariant, string>;

type BadgeProps = useRender.ComponentProps<"span"> & { variant?: BadgeVariant };

/** Нейтральная метка (формат, флаг, «MFA»). Статус с тоном — это `StatusBadge`. */
export function Badge({ className, variant = "default", render, ...props }: BadgeProps) {
  return useRender({
    defaultTagName: "span",
    props: mergeProps<"span">(
      {
        className: cx(
          "inline-flex h-5 w-fit shrink-0 items-center justify-center gap-1 overflow-hidden rounded-full border border-transparent px-2 py-0.5 text-caption font-semibold whitespace-nowrap [&>svg]:pointer-events-none [&>svg]:size-3!",
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
