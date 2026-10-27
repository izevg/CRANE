import { vi } from "vitest";

type Listener = (event: MediaQueryListEvent) => void;

export type MediaQueryMock = {
  /** Меняет результат запроса и оповещает подписчиков. */
  setMatches: (query: string, matches: boolean) => void;
  listenerCount: (query: string) => number;
};

/**
 * jsdom не реализует matchMedia, глобальная заглушка в setup.ts ничему не соответствует.
 * Тест, которому нужен конкретный запрос, ставит мок явно; `initial` — запросы, которые совпадают сразу.
 */
export function stubMatchMedia(initial: string[] = []): MediaQueryMock {
  const state = new Map<string, boolean>(initial.map((query) => [query, true]));
  const listeners = new Map<string, Set<Listener>>();

  vi.stubGlobal("matchMedia", (query: string) => {
    const set = listeners.get(query) ?? new Set<Listener>();
    listeners.set(query, set);
    return {
      get matches() {
        return state.get(query) ?? false;
      },
      media: query,
      addEventListener: (_type: string, listener: Listener) => set.add(listener),
      removeEventListener: (_type: string, listener: Listener) => set.delete(listener),
    };
  });

  return {
    setMatches(query, matches) {
      state.set(query, matches);
      for (const listener of listeners.get(query) ?? []) {
        listener({ matches, media: query } as MediaQueryListEvent);
      }
    },
    listenerCount: (query) => listeners.get(query)?.size ?? 0,
  };
}
