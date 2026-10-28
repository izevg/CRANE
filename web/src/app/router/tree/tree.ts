import { createRootRouteWithContext, createRoute, redirect } from "@tanstack/react-router";
import { loginRoute } from "~/pages/login";
import { notFoundRoute } from "~/pages/not-found";
import { statusRoute } from "~/pages/status";
import { getGetMeQueryOptions } from "~/shared/api";
import { BareLayout } from "../layout/BareLayout";
import { RootLayout } from "../layout/RootLayout";
import { ShellLayout } from "../layout/ShellLayout";
import { ShellError } from "../states/ShellError";
import type { RouterContext } from "./context";

const root = createRootRouteWithContext<RouterContext>()({ component: RootLayout });

// Пока нет экрана поиска (он появится вместе с каталогом), «/» ведёт на единственный готовый раздел.
// redirect выбрасывается в beforeLoad: профиль ради этого не запрашивается, а запись истории заменяется.
const index = createRoute({
  getParentRoute: () => root,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/status", replace: true });
  },
});

// Страницы без шелла (архетип K): карточка по центру.
const bare = createRoute({ getParentRoute: () => root, id: "bare", component: BareLayout });
const login = createRoute({
  getParentRoute: () => bare,
  path: "login",
  ...loginRoute,
});

// Порядок решений: GET /me → сопоставление маршрута (404) → данные маршрута. beforeLoad идёт до loader'ов
// дочерних маршрутов, поэтому страница не запрашивает данные у неавторизованного пользователя.
const shell = createRoute({
  getParentRoute: () => root,
  id: "shell",
  beforeLoad: async ({ context }) => {
    await context.queryClient.ensureQueryData(getGetMeQueryOptions());
  },
  component: ShellLayout,
  errorComponent: ShellError,
  // loader, бросивший notFound(), попадает на тот же экран, что и адрес без маршрута
  notFoundComponent: notFoundRoute.component,
});
const status = createRoute({
  getParentRoute: () => shell,
  path: "status",
  ...statusRoute,
});
const notFound = createRoute({ getParentRoute: () => shell, path: "$", ...notFoundRoute });

export const routeTree = root.addChildren([
  index,
  bare.addChildren([login]),
  shell.addChildren([status, notFound]),
]);
