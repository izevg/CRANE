import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query";
import { ApiProblem } from "~/shared/api";
import { retryDelay, shouldRetry } from "./retry";
import { sessionEnded } from "./sessionEnded";

/** Только что загруженное loader'ом не перечитывается при монтировании страницы. */
const STALE_MS = 5000;

function endSession(error: unknown): void {
  // step-up глобально не обрабатывается: его ловит мутация, показывающая диалог подтверждения
  if (error instanceof ApiProblem && error.kind === "unauthenticated") {
    sessionEnded.set(true);
  }
}

/**
 * Клиент запросов приложения. Глобально обрабатывается только завершение сессии: кэш поднимает флаг для баннера,
 * а переход на вход делает ошибка маршрута. После любой успешной мутации перечитываются активные запросы,
 * неактивные помечаются устаревшими.
 */
export function createQueryClient(): QueryClient {
  const client: QueryClient = new QueryClient({
    queryCache: new QueryCache({ onError: endSession }),
    mutationCache: new MutationCache({
      onSuccess: (): Promise<void> => client.invalidateQueries(),
      onError: endSession,
    }),
    defaultOptions: { queries: { retry: shouldRetry, retryDelay, staleTime: STALE_MS } },
  });
  return client;
}
