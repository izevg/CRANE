import { msg } from "@lingui/core/macro";
import type { QueryClient } from "@tanstack/react-query";
import { getGetStatusQueryOptions } from "~/shared/api";
import { lazyPage } from "~/shared/lib/route";

/**
 * Описание маршрута страницы «Статус системы» (E15). Путь и родителя задаёт app. Лёгкая часть (loader, заголовок)
 * живёт в стартовом чанке, тяжёлая (экран) — за `lazyRouteComponent`: граница ленивости — свойство страницы.
 */
export const statusRoute = {
  staticData: { title: msg`Статус системы` },
  // Те же опции, что у useQuery на странице: loader кладёт данные в кэш, экран читает их без второго запроса.
  loader: ({ context }: { context: { queryClient: QueryClient } }) =>
    context.queryClient.ensureQueryData(getGetStatusQueryOptions()),
  component: lazyPage(() => import("../ui/StatusPage"), "StatusPage"),
};
