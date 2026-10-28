import type { MessageDescriptor } from "@lingui/core";
import { msg, plural } from "@lingui/core/macro";
import type { TextFormat } from "~/shared/lib/text";

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

/** Потолок видимого счётчика: выше показываем «9+», точное число остаётся в доступном имени. */
const BADGE_MAX = 9;
const BADGE_OVERFLOW = "9+";

/** «3 непрочитанных», «1 непрочитанное». Возвращает описание сообщения: перевод делает UI на активной локали. */
export function unreadPhrase(count: number): MessageDescriptor {
  return msg`${plural(count, {
    one: "# непрочитанное",
    few: "# непрочитанных",
    many: "# непрочитанных",
    other: "# непрочитанных",
  })}`;
}

/** Имя кнопки: счётчик — часть имени (MUST), а не только значок. Целое сообщение, а не склейка. */
export function bellLabel(unread: number): MessageDescriptor {
  return unread > 0
    ? msg`${plural(unread, {
        one: "Уведомления, # непрочитанное",
        few: "Уведомления, # непрочитанных",
        many: "Уведомления, # непрочитанных",
        other: "Уведомления, # непрочитанных",
      })}`
    : msg`Уведомления`;
}

export function bellBadge(unread: number, format: TextFormat): string {
  return unread > BADGE_MAX ? BADGE_OVERFLOW : format.number(unread);
}
