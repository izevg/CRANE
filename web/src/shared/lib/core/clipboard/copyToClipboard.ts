import { i18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { announce } from "~/shared/lib/a11y";

/**
 * Копирует текст и объявляет «Скопировано» через общий live-регион.
 * Возвращает `false`, если буфер недоступен или запись отклонена: сообщить об ошибке должен вызывающий.
 */
export async function copyToClipboard(value: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(value);
  } catch {
    return false;
  }
  // вне рендера: сообщение переводится в момент вызова, реактивность не нужна
  announce(i18n._(msg`Скопировано`));
  return true;
}
