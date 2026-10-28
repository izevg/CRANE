import { QueryClientProvider } from "@tanstack/react-query";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { LiveRegion } from "~/shared/lib/a11y";
import { I18nProvider, i18n, initLocale } from "~/shared/lib/i18n";
import { initColorScheme } from "~/shared/lib/state";
import "~/shared/styles";
import { Toaster, TooltipProvider } from "~/shared/ui/primitives";
import { createQueryClient } from "../query/queryClient";
import { createAppRouter } from "../router/tree/router";
import "./index.css";

const root = document.querySelector("#root");
if (root === null) {
  throw new Error("Корневой элемент #root не найден");
}

initColorScheme();
// Каталог загружается до первого рендера: интерфейс не мигает языком по умолчанию.
await initLocale();

const queryClient = createQueryClient();
const router = createAppRouter({ queryClient });

createRoot(root).render(
  <StrictMode>
    <I18nProvider i18n={i18n}>
      <QueryClientProvider client={queryClient}>
        <TooltipProvider>
          <RouterProvider router={router} />
          <LiveRegion />
          <Toaster />
        </TooltipProvider>
      </QueryClientProvider>
    </I18nProvider>
  </StrictMode>,
);
