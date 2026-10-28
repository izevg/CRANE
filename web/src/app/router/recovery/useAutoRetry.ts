import { useEffect } from "react";
import { appBackoff, type Backoff } from "./backoff";

/**
 * Автоповтор, пока показано «нет соединения» или «сервер не отвечает»: по расписанию `backoff` и сразу по событию
 * `online`. Повтор перезапускает маршрут. Обратного отсчёта на экране нет (в live-регион он бы писал каждую секунду);
 * таймер и слушатель снимаются при размонтировании. `retry` должен быть стабильным.
 */
export function useAutoRetry(
  active: boolean,
  retry: () => void,
  retryAfterMs?: number,
  backoff: Backoff = appBackoff,
): void {
  useEffect(() => {
    if (!active) {
      return undefined;
    }
    const timer = setTimeout(retry, backoff.next(Date.now(), retryAfterMs));
    const onOnline = (): void => {
      backoff.reset();
      retry();
    };
    window.addEventListener("online", onOnline);
    return () => {
      clearTimeout(timer);
      window.removeEventListener("online", onOnline);
    };
  }, [active, retry, retryAfterMs, backoff]);
}
