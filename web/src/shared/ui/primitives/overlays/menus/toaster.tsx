import { CircleCheckIcon, InfoIcon, OctagonXIcon, TriangleAlertIcon } from "lucide-react";
import type { CSSProperties } from "react";
import { Toaster as Sonner } from "sonner";
import { useColorMode } from "~/shared/lib/state";

const VISIBLE_TOASTS = 3;

const STYLE = {
  "--normal-bg": "var(--popover)",
  "--normal-text": "var(--popover-foreground)",
  "--normal-border": "var(--border)",
  "--border-radius": "var(--radius)",
} as CSSProperties;

/** Контейнер тостов справа снизу, не больше трёх видимых. Монтируется один раз в корне; показывает `showToast`. */
export function Toaster() {
  const { resolved } = useColorMode();
  return (
    <Sonner
      theme={resolved}
      position="bottom-right"
      visibleToasts={VISIBLE_TOASTS}
      className="toaster group"
      icons={{
        success: <CircleCheckIcon className="size-4" />,
        info: <InfoIcon className="size-4" />,
        warning: <TriangleAlertIcon className="size-4" />,
        error: <OctagonXIcon className="size-4" />,
      }}
      style={STYLE}
    />
  );
}
