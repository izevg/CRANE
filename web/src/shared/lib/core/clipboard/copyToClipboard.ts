import { announce } from "~/shared/lib/a11y";

const COPIED = "Скопировано";

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
  announce(COPIED);
  return true;
}
