import { QueryClientProvider } from "@tanstack/react-query";
import { createMemoryHistory, RouterProvider } from "@tanstack/react-router";
import { render } from "@testing-library/react";
import { vi } from "vitest";
import { createQueryClient } from "~/app/query/queryClient";
import { createAppRouter } from "~/app/router/tree/router";
import { TooltipProvider } from "~/shared/ui/primitives";
import { stubMatchMedia } from "~test/matchMedia";

const WIDE = ["(min-width: 768px)", "(min-width: 1024px)"];

/** Приложение целиком (дерево маршрутов и клиент запросов) на памяти: адрес задаёт тест. */
export function renderApp(url: string) {
  stubMatchMedia(WIDE);
  // jsdom не умеет прокрутку, а роутер восстанавливает её при переходах
  vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
  const history = createMemoryHistory({ initialEntries: [url] });
  const queryClient = createQueryClient();
  const router = createAppRouter({ queryClient, history });
  const view = render(
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
  return { ...view, router, queryClient, history };
}
