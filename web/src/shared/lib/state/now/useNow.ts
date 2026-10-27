import { useCallback, useSyncExternalStore } from "react";

export const MINUTE_MS = 60_000;

/** Допустимая устарелость снимка, пока у часов нет подписчиков и таймер не идёт. */
const FRESH_MS = 1000;

type Clock = {
  now: number;
  timer: ReturnType<typeof setInterval> | undefined;
  readonly listeners: Set<() => void>;
};

/** Часы по интервалу: все подписчики одного интервала делят один таймер. */
const clocks = new Map<number, Clock>();

function clockFor(intervalMs: number): Clock {
  let clock = clocks.get(intervalMs);
  if (clock === undefined) {
    clock = { now: Date.now(), timer: undefined, listeners: new Set() };
    clocks.set(intervalMs, clock);
  }
  return clock;
}

function subscribeClock(intervalMs: number, listener: () => void): () => void {
  const clock = clockFor(intervalMs);
  clock.listeners.add(listener);
  if (clock.timer === undefined) {
    clock.now = Date.now();
    clock.timer = setInterval(() => {
      clock.now = Date.now();
      for (const notify of clock.listeners) {
        notify();
      }
    }, intervalMs);
  }
  return () => {
    clock.listeners.delete(listener);
    if (clock.listeners.size === 0 && clock.timer !== undefined) {
      clearInterval(clock.timer);
      clock.timer = undefined;
    }
  };
}

function snapshot(intervalMs: number): number {
  const clock = clockFor(intervalMs);
  // без подписчиков таймер не идёт: освежаем снимок, чтобы первый кадр не показывал устаревшее время
  if (clock.timer === undefined && Date.now() - clock.now > FRESH_MS) {
    clock.now = Date.now();
  }
  return clock.now;
}

/**
 * Текущее время (мс), обновляемое раз в `intervalMs`. Подписчики одного интервала делят один таймер;
 * когда последний отписался, таймер останавливается.
 */
export function useNow(intervalMs: number = MINUTE_MS): number {
  const subscribe = useCallback(
    (listener: () => void) => subscribeClock(intervalMs, listener),
    [intervalMs],
  );
  return useSyncExternalStore(
    subscribe,
    () => snapshot(intervalMs),
    () => snapshot(intervalMs),
  );
}
