import type { SearchItem } from "~/shared/api";

const SIMILAR_REL = "crane:similar-to";

/**
 * `href` ссылки `crane:similar-to` из HAL элемента поиска: это адрес API, а не маршрут SPA, поэтому читает его
 * страница и сама строит маршрут для `typosquatHref` карточки. Нет ссылки — настоящего пакета нет в видимых
 * репозиториях; битая ссылка (не объект или без строкового `href`) считается отсутствующей.
 */
export function similarToHref(item: Pick<SearchItem, "_links">): string | undefined {
  const link: unknown = item._links[SIMILAR_REL];
  if (typeof link !== "object" || link === null || !("href" in link)) {
    return undefined;
  }
  return typeof link.href === "string" ? link.href : undefined;
}
