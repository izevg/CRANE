import { msg } from "@lingui/core/macro";

/** Заголовок документа для 404: одинаковый для несуществующего и скрытого, без имени из URL. */
export const notFoundRoute = {
  staticData: { title: msg`Не найдено или нет доступа` },
  load: () => import("../ui/NotFoundPage").then((module) => module.NotFoundPage),
};
