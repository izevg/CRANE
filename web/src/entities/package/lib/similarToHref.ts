import { halHref, type SearchItem } from "~/shared/api";

/**
 * `href` ссылки `crane:similar-to` из HAL элемента поиска: читает страница и сама строит маршрут для
 * `typosquatHref` карточки. Нет ссылки — настоящего пакета нет в видимых репозиториях.
 */
export function similarToHref(item: Pick<SearchItem, "_links">): string | undefined {
  return halHref(item._links, "crane:similar-to");
}
