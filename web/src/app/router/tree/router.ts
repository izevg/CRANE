import type { MessageDescriptor } from "@lingui/core";
import type { QueryClient } from "@tanstack/react-query";
import { createRouter, type RouterHistory } from "@tanstack/react-router";
import { NotFoundPage } from "../states/notFound";
import { PendingMain } from "../states/PendingMain";
import { RouteError } from "../states/RouteError";
import { routeTree } from "./tree";

/** Ожидание короче порога не показывается вовсе, показанное держится не меньше минимума: без мигания. */
const PENDING_MS = 300;
const PENDING_MIN_MS = 500;

export function createAppRouter(options: { queryClient: QueryClient; history?: RouterHistory }) {
  return createRouter({
    routeTree,
    context: { queryClient: options.queryClient },
    ...(options.history ? { history: options.history } : {}),
    // Данные и чанк страницы подгружаются по наведению и фокусу (loader с ensureQueryData).
    defaultPreload: "intent",
    defaultPendingMs: PENDING_MS,
    defaultPendingMinMs: PENDING_MIN_MS,
    defaultPendingComponent: PendingMain,
    defaultErrorComponent: RouteError,
    defaultNotFoundComponent: NotFoundPage,
    scrollRestoration: true,
  });
}

type AppRouter = ReturnType<typeof createAppRouter>;

declare module "@tanstack/react-router" {
  interface Register {
    router: AppRouter;
  }
  interface StaticDataRouteOption {
    /** Заголовок раздела: дескриптор каталога, переводится при показе. */
    title?: MessageDescriptor;
  }
}
