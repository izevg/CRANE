import type { LoginSearch } from "~/pages/login";
import type { SessionEnded } from "./queryClient";

/**
 * Сессия завершилась в фоне (опрос, мутация): страницу не покидаем, показывается баннер. Флаг живёт в памяти:
 * после перезагрузки сессия проверяется заново.
 */
let ended = false;
const listeners = new Set<() => void>();

export function isSessionEnded(): boolean {
  return ended;
}

export function markSessionEnded(): void {
  if (ended) {
    return;
  }
  ended = true;
  for (const listener of listeners) {
    listener();
  }
}

export function subscribeSessionEnded(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

type Deps = {
  /** Адрес, на котором пользователь сейчас (или куда идёт). */
  currentHref: () => string;
  /** Переход на /login с заменой записи истории. */
  toLogin: (search: LoginSearch) => void;
  /** Разбор параметров в безопасную форму (причина из белого списка, redirect — путь того же источника). */
  parse: (raw: Record<string, unknown>) => LoginSearch;
  markEnded: () => void;
};

/** На переходе — replace на /login?reason=&redirect=, в фоне — баннер без ухода со страницы. */
export function handleSessionEnded(event: SessionEnded, deps: Deps): void {
  if (event.background) {
    deps.markEnded();
    return;
  }
  deps.toLogin(deps.parse({ reason: event.reason, redirect: deps.currentHref() }));
}
