import type { CSSProperties } from "react";
import { Toaster as Sonner } from "sonner";
import { useColorMode } from "~/shared/lib/state";
import { toneGlyph } from "../../display/tone/tone";

const VISIBLE_TOASTS = 3;

const STYLE = {
  "--normal-bg": "var(--popover)",
  "--normal-text": "var(--popover-foreground)",
  "--normal-border": "var(--border)",
  "--border-radius": "var(--radius)",
} as CSSProperties;

const SuccessGlyph = toneGlyph("success");
const InfoGlyph = toneGlyph("info");
const WarningGlyph = toneGlyph("warning");
const ErrorGlyph = toneGlyph("error");

const ICONS = {
  success: <SuccessGlyph className="size-4" />,
  info: <InfoGlyph className="size-4" />,
  warning: <WarningGlyph className="size-4" />,
  error: <ErrorGlyph className="size-4" />,
};

/** Контейнер тостов справа снизу, не больше трёх видимых. Монтируется один раз в корне; показывает `showToast`. */
export function Toaster() {
  const { resolved } = useColorMode();
  return (
    <Sonner
      theme={resolved}
      position="bottom-right"
      visibleToasts={VISIBLE_TOASTS}
      className="toaster group"
      icons={ICONS}
      style={STYLE}
    />
  );
}
