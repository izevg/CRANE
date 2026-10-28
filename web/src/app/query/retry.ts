import { ApiProblem } from "~/shared/api";

const MAX_RETRIES = 2;
const STATUS_TIMEOUT = 408;
const STATUS_TOO_MANY_REQUESTS = 429;
const STATUS_SERVER_ERROR = 500;
const BASE_DELAY_MS = 1000;
const MAX_DELAY_MS = 30_000;

function isRetryable(error: ApiProblem): boolean {
  switch (error.kind) {
    case "network":
      return true;
    case "problem":
      return (
        error.status >= STATUS_SERVER_ERROR ||
        error.status === STATUS_TIMEOUT ||
        // 429 повторяем только по явному Retry-After: без него время ожидания придумали бы мы
        (error.status === STATUS_TOO_MANY_REQUESTS && error.retryAfterMs !== undefined)
      );
    default:
      // 401, step-up, CSRF и «раздел ещё заглушен» повтором не лечатся
      return false;
  }
}

/** Повтор запроса Query: сеть и 5xx — не больше двух раз; 4xx не повторяем, кроме 408 и 429 с Retry-After. */
export function shouldRetry(failureCount: number, error: unknown): boolean {
  return failureCount < MAX_RETRIES && error instanceof ApiProblem && isRetryable(error);
}

/** Пауза перед повтором: Retry-After сервера, иначе экспоненциальная с потолком. */
export function retryDelay(failureCount: number, error: unknown): number {
  if (error instanceof ApiProblem && error.retryAfterMs !== undefined) {
    return error.retryAfterMs;
  }
  return Math.min(BASE_DELAY_MS * 2 ** failureCount, MAX_DELAY_MS);
}
