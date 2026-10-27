import type { MessageDescriptor } from "@lingui/core";
import { msg, plural } from "@lingui/core/macro";

// CodeBlock
export const CODE_LABEL = msg`Код`;

// DataTable
export const TABLE_LOADING = msg`Загрузка`;
export const TABLE_ACTIONS = msg`Действия`;
export const TABLE_SELECT_ALL = msg`Выбрать все загруженные`;

// DiffViewer
export const DIFF_BEFORE = msg`Было`;
export const DIFF_AFTER = msg`Стало`;
export const DIFF_PATH = msg`Поле`;
export const DIFF_CHANGE = msg`Изменение`;
export const DIFF_CONTENT = msg`Содержимое`;
export const DIFF_ADDED = msg`добавлено`;
export const DIFF_REMOVED = msg`удалено`;

/** Изменённое поле одной фразой для скринридера: порядок слов принадлежит переводу. */
export function diffChanged(before: string, after: string): MessageDescriptor {
  return msg`изменено: было ${before}, стало ${after}`;
}

// JsonViewer
export const JSON_ROOT = msg`корень`;
export const JSON_EXPAND_ALL = msg`Развернуть всё`;
export const JSON_COLLAPSE_ALL = msg`Свернуть всё`;
export const JSON_COPY = msg`Копировать JSON`;

export function jsonCollapse(name: string): MessageDescriptor {
  return msg`Свернуть ${name}`;
}

export function jsonExpand(name: string): MessageDescriptor {
  return msg`Развернуть ${name}`;
}

/** «3 элемента» / «12 ключей»: свёрнутый контейнер. */
export function jsonSummary(container: "array" | "object", size: number): MessageDescriptor {
  return container === "array"
    ? msg`${plural(size, { one: "# элемент", few: "# элемента", many: "# элементов", other: "# элемента" })}`
    : msg`${plural(size, { one: "# ключ", few: "# ключа", many: "# ключей", other: "# ключа" })}`;
}

// Markdown
export const MARKDOWN_LOADING = msg`Загрузка текста`;
export const MARKDOWN_NEW_TAB = msg` (откроется в новой вкладке)`;
