import type { QueryClient } from "@tanstack/react-query";
import type { RouterHistory } from "@tanstack/react-router";
import { loginRoute } from "~/pages/login";
import { createQueryClient } from "../../query/queryClient";
import { handleSessionEnded, markSessionEnded } from "../../query/sessionEnded";
import { createAppRouter } from "./router";

/**
 * Клиент запросов и роутер, связанные завершением сессии. Роутеру клиент нужен в контексте, а обработчику
 * 401 нужен роутер (уйти на /login): связь через замыкание.
 */
export function createApp(options: { history?: RouterHistory } = {}): {
  queryClient: QueryClient;
  router: ReturnType<typeof createAppRouter>;
} {
  const queryClient = createQueryClient((event) =>
    handleSessionEnded(event, {
      currentHref: () => router.latestLocation.href,
      toLogin: (search) => {
        void router.navigate({ to: "/login", search, replace: true });
      },
      parse: loginRoute.validateSearch,
      markEnded: markSessionEnded,
    }),
  );
  const router = createAppRouter({
    queryClient,
    ...(options.history ? { history: options.history } : {}),
  });
  return { queryClient, router };
}
