// i18n: не мигрирован
import type { MessageDescriptor } from "@lingui/core";
import { msg, plural } from "@lingui/core/macro";
import type { Action } from "~/shared/ui/actions";

export type NavigationEntry = { id: string; label: string; href: string; hint?: string };

export type PackageHit = {
  id: string;
  name: string;
  format: string;
  latest?: string;
  href: string;
};

/** Значение пункта списка: тип нужен, чтобы выбор вёл себя по-разному (переход, пакет, действие). */
export type PaletteOption =
  | { kind: "navigation"; key: string; entry: NavigationEntry }
  | { kind: "package"; key: string; entry: PackageHit }
  | { kind: "action"; key: string; entry: Action };

/** Группа в форме, которую понимает Base UI: `value` — заголовок, `items` — пункты. */
export type PaletteGroup = { value: string; items: PaletteOption[] };

const GROUP_NAVIGATION = "Перейти";
const GROUP_PACKAGES = "Пакеты";
const GROUP_ACTIONS = "Действия";

function matches(text: string, needle: string): boolean {
  return text.toLocaleLowerCase("ru").includes(needle);
}

type BuildInput = {
  query: string;
  navigation: readonly NavigationEntry[];
  packages: readonly PackageHit[];
  actions: readonly Action[];
};

/**
 * «Перейти» и «Действия» фильтруются локально по вводу. «Пакеты» не фильтруются: их уже отобрал сервер,
 * и совпадение может быть не буквальным (синоним, нормализация имени); локальный фильтр спрятал бы валидный ответ.
 * Пустые группы не попадают в список.
 */
export function buildGroups({ query, navigation, packages, actions }: BuildInput): PaletteGroup[] {
  const needle = query.trim().toLocaleLowerCase("ru");
  const keep = (text: string) => needle === "" || matches(text, needle);
  const groups: PaletteGroup[] = [
    {
      value: GROUP_NAVIGATION,
      items: navigation
        .filter((entry) => keep(entry.label))
        .map((entry) => ({ kind: "navigation", key: `nav:${entry.id}`, entry })),
    },
    {
      value: GROUP_PACKAGES,
      items: packages.map((entry) => ({ kind: "package", key: `pkg:${entry.id}`, entry })),
    },
    {
      value: GROUP_ACTIONS,
      items: actions
        .filter((action) => keep(action.label))
        .map((entry) => ({ kind: "action", key: `act:${entry.label}`, entry })),
    },
  ];
  return groups.filter((group) => group.items.length > 0);
}

/** Строка пункта для Base UI (`itemToStringValue`). */
export function optionText(option: PaletteOption): string {
  if (option.kind === "package") {
    return option.entry.name;
  }
  return option.entry.label;
}

/** Что делает выбор: переход по ссылке или вызов обработчика. */
export function pickTarget(option: PaletteOption): { href: string } | { run: () => void } {
  if (option.kind !== "action") {
    return { href: option.entry.href };
  }
  const { run } = option.entry;
  return "href" in run ? { href: run.href } : { run: run.onSelect };
}

/**
 * Итог поиска для скринридера (в живой регион `Status`). Пустой ввод не объявляется: при открытии палитры
 * шум не нужен. Пустой результат не дублируем: «Ничего не найдено» объявляет сам `Autocomplete.Empty`.
 */
export function resultsStatus({
  query,
  groups,
}: {
  query: string;
  groups: readonly PaletteGroup[];
}): MessageDescriptor | null {
  const total = groups.reduce((sum, group) => sum + group.items.length, 0);
  if (query.trim() === "" || total === 0) {
    return null;
  }
  return msg`${plural(total, {
    one: "Результатов: #",
    few: "Результатов: #",
    many: "Результатов: #",
    other: "Результатов: #",
  })}`;
}
