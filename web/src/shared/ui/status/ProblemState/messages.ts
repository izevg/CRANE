import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";

// Заголовки по kind — локальный словарь: серверный `title` для них не показывается (язык сервера не зависит от интерфейса).
export const NOT_FOUND_TITLE = msg`Не найдено или нет доступа`;
export const NOT_FOUND_HINT = msg`Проверьте адрес или имя пакета`;
export const FORBIDDEN_TITLE = msg`Недостаточно прав`;
export const OFFLINE_TITLE = msg`Нет соединения`;
export const OFFLINE_HINT = msg`Проверьте сеть — данные обновятся, когда связь вернётся`;
export const SESSION_TITLE = msg`Сессия завершена`;
export const SESSION_HINT = msg`Войдите снова, чтобы продолжить работу`;

export const REQUEST_ID = msg`Идентификатор запроса:`;
export const COPY_REQUEST_ID = msg`Скопировать идентификатор запроса`;

export function ruleLine(ruleId: string): MessageDescriptor {
  return msg`Правило: ${ruleId}`;
}

export function contactLine(contact: string): MessageDescriptor {
  return msg`Контакт: ${contact}`;
}
