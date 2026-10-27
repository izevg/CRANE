import { MotionConfig } from "motion/react";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { LiveRegion } from "~/shared/lib/a11y";
import { initColorScheme } from "~/shared/lib/state";
import "~/shared/styles";
import { Toaster, TooltipProvider } from "~/shared/ui/primitives";
import { App } from "./App";

const root = document.querySelector("#root");
if (root === null) {
  throw new Error("Корневой элемент #root не найден");
}

initColorScheme();

createRoot(root).render(
  <StrictMode>
    <MotionConfig reducedMotion="user">
      <TooltipProvider>
        <App />
        <LiveRegion />
        <Toaster />
      </TooltipProvider>
    </MotionConfig>
  </StrictMode>,
);
