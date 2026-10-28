import { Switch as SwitchPrimitive } from "@base-ui/react/switch";
import { cx, type WithClassName } from "~/shared/lib/core";

type SwitchSize = "sm" | "default";

const SIZE = {
  default: "h-5 w-9",
  sm: "h-4 w-7",
} satisfies Record<SwitchSize, string>;

const THUMB = {
  default: "size-4",
  sm: "size-3",
} satisfies Record<SwitchSize, string>;

type SwitchProps = WithClassName<SwitchPrimitive.Root.Props> & { size?: SwitchSize };

// after:-inset расширяет зону нажатия до ≥ 24 px по высоте (WCAG 2.5.8).
export function Switch({ className, size = "default", ...props }: SwitchProps) {
  return (
    <SwitchPrimitive.Root
      data-slot="switch"
      className={cx(
        "peer relative inline-flex shrink-0 items-center rounded-full border border-transparent transition-colors duration-(--dur-fast) after:absolute after:-inset-x-3 after:-inset-y-2 data-checked:bg-primary data-unchecked:bg-input data-disabled:cursor-not-allowed data-disabled:bg-surface-high forced-colors:border-[ButtonText] forced-colors:data-checked:bg-[Highlight] forced-colors:data-unchecked:bg-[Canvas]",
        SIZE[size],
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot="switch-thumb"
        className={cx(
          "pointer-events-none block rounded-full bg-card transition-transform duration-(--dur-fast) data-checked:translate-x-[calc(100%-2px)] data-checked:bg-primary-foreground data-unchecked:translate-x-0.5 forced-colors:bg-[ButtonText] forced-colors:data-checked:bg-[HighlightText]",
          THUMB[size],
        )}
      />
    </SwitchPrimitive.Root>
  );
}
