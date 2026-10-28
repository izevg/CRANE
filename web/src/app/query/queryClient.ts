import { MutationCache, QueryCache, QueryClient } from "@tanstack/react-query";
import { ApiProblem } from "~/shared/api";
import { retryDelay, shouldRetry } from "./retry";

/** Только что загруженное loader'ом не перечитывается при монтировании страницы. */
const STALE_MS = 5000;

export type SessionEnded = {
  /** Пользователь не на переходе: опрос или мутация. Страницу покидать нельзя, показывается баннер. */
  background: boolean;
  reason?: string;
};

type Handler = (event: SessionEnded) => void;

function sessionEndedOf(error: unknown, background: boolean): SessionEnded | undefined {
  if (!(error instanceof ApiProblem && error.kind === "unauthenticated")) {
    // step-up глобально не обрабатывается: его ловит мутация, показывающая диалог подтверждения
    return undefined;
  }
  return error.reason === undefined ? { background } : { background, reason: error.reason };
}

/**
 * Клиент запросов приложения. Глобально обрабатывается только завершение сессии: на переходе (запрос без
 * наблюдателей — это loader) уводим на вход, в фоне (есть наблюдатели, либо мутация) только сообщаем.
 * После любой успешной мутации перечитываются активные запросы, неактивные помечаются устаревшими.
 */
export function createQueryClient(onSessionEnded: Handler): QueryClient {
  function report(error: unknown, background: boolean): void {
    const event = sessionEndedOf(error, background);
    if (event) {
      onSessionEnded(event);
    }
  }
  const client: QueryClient = new QueryClient({
    queryCache: new QueryCache({
      onError: (error, query) => report(error, query.getObserversCount() > 0),
    }),
    mutationCache: new MutationCache({
      onSuccess: (): Promise<void> => client.invalidateQueries(),
      onError: (error) => report(error, true),
    }),
    defaultOptions: { queries: { retry: shouldRetry, retryDelay, staleTime: STALE_MS } },
  });
  return client;
}
