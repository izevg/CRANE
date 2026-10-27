import { i18n } from "~/shared/lib/i18n";
import { type TextFormat, textFormat } from "~/shared/lib/text";

/** Форматтеры активной локали в местах, где нет хука (ячейки таблиц, фабрики данных); где хук доступен — `useTextFormat()`. */
export function activeFormat(): TextFormat {
  return textFormat(i18n.locale);
}
