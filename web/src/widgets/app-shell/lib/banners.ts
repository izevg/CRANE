/** Больше двух баннеров одновременно — шум в глобальной зоне (patterns §25). */
const MAX_VISIBLE = 2;
const TEXT_SEPARATOR = ". ";

type BannerLike = { id: string; text: string; urgent?: true };

/** Видимые баннеры: два самых важных, остальные только после раскрытия. */
export function visibleBanners<T>(banners: readonly T[], expanded: boolean): readonly T[] {
  return expanded ? banners : banners.slice(0, MAX_VISIBLE);
}

/** Сколько баннеров скрыто за кнопкой «Ещё N». */
export function hiddenCount(total: number): number {
  return Math.max(total - MAX_VISIBLE, 0);
}

/**
 * Что объявить скринридеру: новые обычные баннеры одной фразой. Срочные (P0) объявляет `role="alert"` у Callout.
 * Возвращает и новый набор виденных id: пропавший из набора id забывается и при возвращении объявится снова.
 */
export function announcementFor(
  banners: readonly BannerLike[],
  seen: ReadonlySet<string>,
): { text: string | null; seen: Set<string> } {
  const texts = banners
    .filter((banner) => !seen.has(banner.id) && banner.urgent !== true)
    .map((banner) => banner.text);
  return {
    text: texts.length > 0 ? texts.join(TEXT_SEPARATOR) : null,
    seen: new Set(banners.map((banner) => banner.id)),
  };
}
