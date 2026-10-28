import type { HalLinks } from "../generated/schemas";

/**
 * `href` ссылки HAL по отношению (`crane:similar-to`, `crane:block`). Это адрес API, а не маршрут SPA: страница
 * сама строит маршрут. UI никогда не собирает маршрут к связанному ресурсу из id. Нет ссылки (ресурса нет в
 * видимых репозиториях или действие не разрешено) и битая ссылка (не объект или без строкового `href`) —
 * одинаково `undefined`.
 */
export function halHref(links: HalLinks, rel: string): string | undefined {
  const link: unknown = links[rel];
  if (typeof link !== "object" || link === null || !("href" in link)) {
    return undefined;
  }
  return typeof link.href === "string" ? link.href : undefined;
}
