import { lazyRouteComponent } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { readStored, writeStored } from "~/shared/lib/state";

/** В это окно вторая перезагрузка из-за сбоя чанка не делается: иначе устаревший деплой зациклил бы страницу. */
export const RELOAD_WINDOW_MS = 10_000;
const RELOAD_KEY = "crane.chunk-reload-at";
const TIMESTAMP = /^\d+$/;

// Сообщения браузеров и Vite о том, что не удалось загрузить код маршрута (Chrome, Firefox, Safari, preload CSS).
const CHUNK_MESSAGES: readonly RegExp[] = [
  /Failed to fetch dynamically imported module/i,
  /error loading dynamically imported module/i,
  /Importing a module script failed/i,
  /Unable to preload CSS/i,
];

export type ChunkAction = "offline" | "reload" | "show";

export function isChunkLoadError(error: unknown): boolean {
  return error instanceof Error && CHUNK_MESSAGES.some((pattern) => pattern.test(error.message));
}

/**
 * Код страницы не загрузился. Своё имя и сообщение нужны, чтобы `lazyRouteComponent` не принял ошибку за
 * «модуль не найден» и не перезагрузил страницу сам: у него нет ни окна защиты от цикла, ни проверки сети.
 */
export class ChunkLoadError extends Error {
  constructor(cause: unknown) {
    super("Page chunk failed to load", { cause });
    this.name = "ChunkLoadError";
  }
}

/**
 * Ленивая страница слайса. Сбой загрузки чанка превращается в `ChunkLoadError`, остальные ошибки (исключение
 * при вычислении модуля) остаются как есть. Ленивый код внутри страницы (палитра, график) через эту функцию не
 * идёт, поэтому его сбой страницу не перезагружает (WCAG 3.3.7).
 */
export function lazyPage(load: () => Promise<() => ReactNode>) {
  return lazyRouteComponent(() =>
    load()
      .then((component) => ({ default: component }))
      .catch((cause: unknown) => {
        throw isChunkLoadError(cause) ? new ChunkLoadError(cause) : cause;
      }),
  );
}

/**
 * Что делать при сбое чанка маршрута. Нет сети: перезагрузка не поможет, показываем «нет соединения».
 * Иначе одна автоматическая перезагрузка; сбой в течение окна после неё — экран ошибки без цикла.
 */
export function chunkAction(state: {
  online: boolean;
  lastReloadAt: number | undefined;
  now: number;
}): ChunkAction {
  if (!state.online) {
    return "offline";
  }
  const recent =
    state.lastReloadAt !== undefined && state.now - state.lastReloadAt < RELOAD_WINDOW_MS;
  return recent ? "show" : "reload";
}

function isTimestamp(value: string): value is string {
  return TIMESTAMP.test(value);
}

export function decideChunkAction(): ChunkAction {
  const stored = readStored("session", RELOAD_KEY, isTimestamp);
  return chunkAction({
    online: navigator.onLine,
    lastReloadAt: stored === undefined ? undefined : Number(stored),
    now: Date.now(),
  });
}

/** Отмечает автоматическую перезагрузку; без хранилища защита от цикла невозможна, поэтому `false`. */
export function markReload(): boolean {
  return writeStored("session", RELOAD_KEY, String(Date.now()));
}
