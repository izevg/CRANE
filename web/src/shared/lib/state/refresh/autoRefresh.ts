/**
 * Пауза автообновления — один флаг на приложение (WCAG 2.2.2): его переключает AutoRefreshToggle, а опрос
 * операций (`shared/api`) читает. Живёт в памяти: после перезагрузки опрос снова идёт, как задумано контрактом.
 */
let paused = false;
const listeners = new Set<() => void>();

export function isAutoRefreshPaused(): boolean {
  return paused;
}

export function setAutoRefreshPaused(value: boolean): void {
  if (paused === value) {
    return;
  }
  paused = value;
  for (const listener of listeners) {
    listener();
  }
}

export function subscribeAutoRefresh(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}
