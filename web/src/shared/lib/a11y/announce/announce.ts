import { useSyncExternalStore } from "react";

/** Пауза между сбросом региона и записью текста: без неё React схлопнет оба обновления в одно. */
const REWRITE_DELAY_MS = 50;

let current = "";
let pending: ReturnType<typeof setTimeout> | undefined;
const listeners = new Set<() => void>();

function set(text: string): void {
  current = text;
  for (const listener of listeners) {
    listener();
  }
}

/**
 * Объявляет сообщение скринридеру через единственный polite live-регион (`<LiveRegion />`).
 * Повтор того же текста объявляется снова: регион сначала очищается, потом пишется текст.
 */
export function announce(text: string): void {
  clearTimeout(pending);
  set("");
  pending = setTimeout(() => {
    pending = undefined;
    set(text);
  }, REWRITE_DELAY_MS);
}

/** Сбрасывает сообщение и отложенную запись: регион, смонтированный заново, не показывает чужой текст. */
export function resetAnnouncement(): void {
  clearTimeout(pending);
  pending = undefined;
  set("");
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Текущее сообщение региона; нужен только `<LiveRegion />`. */
export function useAnnouncement(): string {
  return useSyncExternalStore(
    subscribe,
    () => current,
    () => "",
  );
}
