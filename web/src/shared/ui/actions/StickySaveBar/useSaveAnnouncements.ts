import { useEffect, useRef } from "react";
import { announce } from "~/shared/lib/a11y";

/**
 * Объявления панели сохранения. Число правок не объявляется на каждое нажатие клавиши: только при первой правке
 * за сессию (пока `dirty` не вернулся к нулю), иначе скринридер читал бы «Изменено: 1, 2, 3…» посреди ввода.
 * Итог сохранения объявляется всегда: от него зависит, что делать дальше.
 */
export function useSaveAnnouncements(dirty: number, changed: string, summary: string | null): void {
  const sessionAnnounced = useRef<boolean>(false);
  useEffect(() => {
    if (dirty === 0) {
      sessionAnnounced.current = false;
      return;
    }
    if (!sessionAnnounced.current) {
      sessionAnnounced.current = true;
      announce(changed);
    }
  }, [dirty, changed]);
  useEffect(() => {
    if (summary !== null) {
      announce(summary);
    }
  }, [summary]);
}
