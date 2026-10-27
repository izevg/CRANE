import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { LiveRegion } from "~/shared/lib/a11y";
import { initColorScheme } from "~/shared/lib/state";
import "~/shared/styles";
import { Toaster, TooltipProvider } from "~/shared/ui/primitives";
import { App } from "./App";
import "./index.css";

const root = document.querySelector("#root");
if (root === null) {
  throw new Error("Корневой элемент #root не найден");
}

initColorScheme();

createRoot(root).render(
  <StrictMode>
    <TooltipProvider>
      <App />
      <LiveRegion />
      <Toaster />
    </TooltipProvider>
  </StrictMode>,
);
