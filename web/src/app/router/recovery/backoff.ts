/** Паузы автоповтора при недоступном сервере (спека §2): 5 → 10 → 30 → 60 с, дальше раз в 60 с. */
const SECOND_MS = 1000;
const PAUSE_S = { first: 5, second: 10, third: 30, last: 60 };
const SCHEDULE_MS = Object.values(PAUSE_S).map((seconds) => seconds * SECOND_MS);
/** Прошло столько после последнего повтора — это уже новый сбой, отсчёт начинается с начала. */
const FORGET_AFTER_S = 180;
const FORGET_AFTER_MS = FORGET_AFTER_S * SECOND_MS;

export type Backoff = {
  /** Пауза до следующего повтора; `retryAfterMs` сервера может её только удлинить. */
  next: (now: number, retryAfterMs?: number) => number;
  /** Сеть вернулась или маршрут сменился: следующий сбой снова начнёт с 5 с. */
  reset: () => void;
};

export function createBackoff(): Backoff {
  let attempt = 0;
  let lastAt = Number.NEGATIVE_INFINITY;
  return {
    next(now, retryAfterMs = 0) {
      if (now - lastAt > FORGET_AFTER_MS) {
        attempt = 0;
      }
      const base = SCHEDULE_MS[Math.min(attempt, SCHEDULE_MS.length - 1)] ?? 0;
      attempt += 1;
      lastAt = now;
      return Math.max(base, retryAfterMs);
    },
    reset() {
      attempt = 0;
      lastAt = Number.NEGATIVE_INFINITY;
    },
  };
}

/** Один счётчик на приложение: экран ошибки пересоздаётся при каждом повторе и сам помнить попытки не может. */
export const appBackoff = createBackoff();
