import { QueryClientProvider } from "@tanstack/react-query";
import { type ReactNode, useState } from "react";
import { createQueryClient } from "~/app/query/queryClient";
import { sessionEnded } from "~/app/query/sessionEnded";

/**
 * Свежий клиент запросов приложения на каждую историю (с его обработкой 401): данные одной истории не видны другой.
 * Повторы выключены, иначе история ошибки ждала бы их; данные считаются устаревшими сразу, поэтому страница при
 * монтировании перечитывает то, что положил loader (на этом построены истории «обновление не удалось»). Флаг «сессия
 * завершена» сбрасывается.
 */
export function ApiStory({ children }: { children: ReactNode }) {
  const [client] = useState(() => {
    sessionEnded.set(false);
    const created = createQueryClient();
    created.setDefaultOptions({
      ...created.getDefaultOptions(),
      queries: { ...created.getDefaultOptions().queries, retry: false, staleTime: 0 },
    });
    return created;
  });
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>;
}
