import type { MessageDescriptor } from "@lingui/core";

const BRAND = "CRANE";
const SEPARATOR = " — ";

type MatchLike = {
  status: string;
  globalNotFound?: boolean | undefined;
  staticData?: { title?: MessageDescriptor | undefined } | undefined;
};

/**
 * Заголовок раздела по совпавшим маршрутам. Если любое совпадение «не найдено» (адрес без маршрута или loader
 * бросил notFound), заголовок один и тот же, чтобы по title нельзя было отличить скрытый ресурс от несуществующего.
 */
export function pageTitleOf(
  matches: readonly MatchLike[],
  notFound: MessageDescriptor,
): MessageDescriptor | undefined {
  if (matches.some((match) => match.status === "notFound" || match.globalNotFound === true)) {
    return notFound;
  }
  return matches.map((match) => match.staticData?.title).findLast((title) => title !== undefined);
}

/** «<Страница> — CRANE»; без страницы — только имя продукта. */
export function documentTitle(page: string | undefined): string {
  return page === undefined ? BRAND : `${page}${SEPARATOR}${BRAND}`;
}
