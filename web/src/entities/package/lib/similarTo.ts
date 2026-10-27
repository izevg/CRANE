import type { SearchItem } from "~/shared/api";

const SIMILAR_REL = "crane:similar-to";

export type SimilarTo = {
  /** Отображаемое имя настоящего пакета. */
  name: string;
  /** PURL настоящего пакета без версии: читается как текст, без моно. */
  purl: string;
  /** `href` ссылки `crane:similar-to` из API. Это не маршрут SPA: маршрут строит страница. Нет ссылки — настоящего пакета нет в видимых репозиториях. */
  apiHref: string | undefined;
};

function hrefOf(link: unknown): string | undefined {
  if (typeof link !== "object" || link === null || !("href" in link)) {
    return undefined;
  }
  return typeof link.href === "string" ? link.href : undefined;
}

/** Цель предупреждения о typosquat у элемента поиска; `null`, если предупреждения нет. */
export function similarTo(item: Pick<SearchItem, "typosquat" | "_links">): SimilarTo | null {
  const { typosquat } = item;
  if (typosquat == null) {
    return null;
  }
  return {
    name: typosquat.similar_to,
    purl: typosquat.similar_to_purl,
    apiHref: hrefOf(item._links[SIMILAR_REL]),
  };
}
