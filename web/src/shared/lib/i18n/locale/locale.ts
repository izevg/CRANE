import { i18n } from "@lingui/core";
import { readStored, writeStored } from "~/shared/lib/state";

export const LOCALES = ["ru", "en"] as const;
export type Locale = (typeof LOCALES)[number];

/** Русский — исходный язык сообщений и язык по умолчанию. */
const DEFAULT_LOCALE: Locale = "ru";
const STORAGE_KEY = "crane.locale";

export function isLocale(value: unknown): value is Locale {
  return typeof value === "string" && (LOCALES as readonly string[]).includes(value);
}

function readStoredLocale(): Locale {
  return readStored("local", STORAGE_KEY, isLocale) ?? DEFAULT_LOCALE;
}

/** Если хранилище недоступно, выбор живёт до перезагрузки. */
function writeStoredLocale(locale: Locale): void {
  writeStored("local", STORAGE_KEY, locale);
}

async function loadMessages(locale: Locale) {
  const catalog =
    locale === "en" ? await import("../locales/en.po") : await import("../locales/ru.po");
  return catalog.messages;
}

/**
 * Загружает каталог, включает локаль и ставит `<html lang>` (от него зависит произношение в скринридере).
 * В хранилище ничего не пишет: так витрина и тесты не трогают выбор пользователя.
 */
export async function activateLocale(locale: Locale): Promise<void> {
  i18n.load(locale, await loadMessages(locale));
  i18n.activate(locale);
  document.documentElement.lang = locale;
}

/** Пользователь выбрал язык: запоминаем выбор и включаем локаль. */
export async function setLocale(locale: Locale): Promise<void> {
  writeStoredLocale(locale);
  await activateLocale(locale);
}

/** Вызывается до первого рендера: читает выбор из хранилища, по умолчанию русский. */
export async function initLocale(): Promise<void> {
  await activateLocale(readStoredLocale());
}
