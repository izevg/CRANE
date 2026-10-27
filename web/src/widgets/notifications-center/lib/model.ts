import { formatNumber, pluralRu } from "~/shared/lib/text";

export type NotificationCategory = "security" | "access" | "platform";

export type NotificationItem = {
  id: string;
  category: NotificationCategory;
  tone: "info" | "warning" | "error" | "success";
  title: string;
  /** ISO 8601. */
  time: string;
  /** Контекст под заголовком: пакет, репозиторий, актор. */
  caption?: string;
  /** Объект уведомления (`crane:target`). */
  href: string;
  unread: boolean;
};

const BELL_LABEL = "Уведомления";
const UNREAD_FORMS = ["непрочитанное", "непрочитанных", "непрочитанных"] as const;

/** Потолок видимого счётчика: выше показываем «9+», точное число остаётся в доступном имени. */
const BADGE_MAX = 9;
const BADGE_OVERFLOW = "9+";

/** «3 непрочитанных», «1 непрочитанное». */
export function unreadPhrase(count: number): string {
  return `${formatNumber(count)} ${pluralRu(count, UNREAD_FORMS)}`;
}

/** Имя кнопки: счётчик — часть имени (MUST), а не только значок. */
export function bellLabel(unread: number): string {
  return unread > 0 ? `${BELL_LABEL}, ${unreadPhrase(unread)}` : BELL_LABEL;
}

export function bellBadge(unread: number): string {
  return unread > BADGE_MAX ? BADGE_OVERFLOW : formatNumber(unread);
}
