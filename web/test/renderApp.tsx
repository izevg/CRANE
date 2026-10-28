import { QueryClientProvider } from "@tanstack/react-query";
import { createMemoryHistory, RouterProvider } from "@tanstack/react-router";
import { render } from "@testing-library/react";
import { vi } from "vitest";
import { createApp } from "~/app/router/tree/app";
import { TooltipProvider } from "~/shared/ui/primitives";
import { stubMatchMedia } from "~test/matchMedia";

const WIDE = ["(min-width: 768px)", "(min-width: 1024px)"];

/** Приложение целиком (дерево маршрутов, клиент, обработчик 401) на памяти: адрес задаёт тест. */
export function renderApp(url: string) {
  stubMatchMedia(WIDE);
  // jsdom не умеет прокрутку, а роутер восстанавливает её при переходах
  vi.spyOn(window, "scrollTo").mockImplementation(() => undefined);
  const history = createMemoryHistory({ initialEntries: [url] });
  const { queryClient, router } = createApp({ history });
  const view = render(
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>
        <RouterProvider router={router} />
      </TooltipProvider>
    </QueryClientProvider>,
  );
  return { ...view, router, queryClient, history };
}
