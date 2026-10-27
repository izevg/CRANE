import type { NotificationCategory } from "./model";

export const ALL_TAB = "all";

export type NotificationTab = typeof ALL_TAB | NotificationCategory;

export function isCategory(value: unknown): value is NotificationCategory {
  return value === "security" || value === "access" || value === "platform";
}

/** Вкладка по категории: без категории открыта «Все». */
export function tabOf(category: NotificationCategory | undefined): NotificationTab {
  return category ?? ALL_TAB;
}

/** Счётчик непрочитанных вкладки: у «Все» общий, у остальных по категории. */
export function tabCount(
  tab: NotificationTab,
  unread: number,
  byCategory: Readonly<Record<NotificationCategory, number>>,
): number {
  return isCategory(tab) ? byCategory[tab] : unread;
}
