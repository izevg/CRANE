import { notFoundRoute } from "~/pages/not-found";
import { lazyPage } from "../chunk/chunk";

/** Один экземпляр на всё приложение: адрес без маршрута, `notFound()` из loader и ответ API 404 рисуют его же. */
export const NotFoundPage = lazyPage(notFoundRoute.load);
