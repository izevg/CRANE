import { readStored, writeStored } from "~/shared/lib/state";

/** В это окно вторая перезагрузка из-за сбоя чанка не делается: иначе устаревший деплой зациклил бы страницу. */
export const RELOAD_WINDOW_MS = 10_000;
const RELOAD_KEY = "crane.chunk-reload-at";
const TIMESTAMP = /^\d+$/;

export type ChunkAction = "offline" | "reload" | "show";

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
