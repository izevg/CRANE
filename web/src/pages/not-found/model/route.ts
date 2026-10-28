import { msg } from "@lingui/core/macro";
import { lazyPage } from "~/shared/lib/route";

/** Заголовок документа для 404: одинаковый для несуществующего и скрытого, без имени из URL. */
export const notFoundRoute = {
  staticData: { title: msg`Не найдено или нет доступа` },
  component: lazyPage(() => import("../ui/NotFoundPage"), "NotFoundPage"),
};
