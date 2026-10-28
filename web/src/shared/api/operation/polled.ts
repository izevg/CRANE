import { isAutoRefreshPaused } from "~/shared/lib/state";
import { ApiProblem } from "../http/problem";
import type { OperationId } from "../stubs/table/types";

const STATUS_POLL_MS = 30_000;

/**
 * Каденс опроса по операциям: свойство контракта (`Cache-Control: private, max-age=<каденс>` в спеке), поэтому
 * стоит рядом с операцией, а не у каждого вызова. Нет записи — операция не опрашивается.
 */
const POLL_MS = { getStatus: STATUS_POLL_MS } satisfies Partial<Record<OperationId, number>>;

const CADENCE: Readonly<Partial<Record<string, number>>> = POLL_MS;

type Polling = { state: { error: unknown } };

/**
 * Добавляет `refetchInterval`. Опрос стоит при паузе и после 401 (сессия завершена: повторы только множат ошибки).
 * Скрытую вкладку обрабатывает сам Query (`refetchIntervalInBackground: false`). Интервал пересчитывается при
 * каждом рендере наблюдателя, поэтому экран с AutoRefreshToggle, перерисовываясь при переключении, возобновляет опрос.
 */
export function polled<T extends { refetchInterval?: unknown }>(
  options: T,
  operationId: string,
  paused: () => boolean = isAutoRefreshPaused,
): T {
  const interval = CADENCE[operationId];
  if (interval === undefined || options.refetchInterval !== undefined) {
    return options;
  }
  const refetchInterval = (query: Polling): number | false =>
    paused() ||
    (query.state.error instanceof ApiProblem && query.state.error.kind === "unauthenticated")
      ? false
      : interval;
  return { ...options, refetchInterval };
}
