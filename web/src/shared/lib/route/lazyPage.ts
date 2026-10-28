import { lazyRouteComponent } from "@tanstack/react-router";

// Сообщения браузеров и Vite о том, что не удалось загрузить код маршрута (Chrome, Firefox, Safari, preload CSS).
const CHUNK_MESSAGES: readonly RegExp[] = [
  /Failed to fetch dynamically imported module/i,
  /error loading dynamically imported module/i,
  /Importing a module script failed/i,
  /Unable to preload CSS/i,
];

export function isChunkLoadError(error: unknown): boolean {
  return error instanceof Error && CHUNK_MESSAGES.some((pattern) => pattern.test(error.message));
}

/**
 * Код страницы не загрузился. Своё имя и сообщение нужны, чтобы `lazyRouteComponent` не принял ошибку за
 * «модуль не найден» и не перезагрузил страницу сам: у него нет ни окна защиты от цикла, ни проверки сети.
 */
export class ChunkLoadError extends Error {
  constructor(cause: unknown) {
    super("Page chunk failed to load", { cause });
    this.name = "ChunkLoadError";
  }
}

/**
 * Ленивая страница слайса: `component: lazyPage(() => import("../ui/StatusPage"), "StatusPage")`. Сбой загрузки
 * чанка превращается в `ChunkLoadError`, остальные ошибки (исключение при вычислении модуля) остаются как есть.
 * Что делать с `ChunkLoadError`, решает приложение. Ленивый код внутри страницы (палитра, график) через эту функцию
 * не идёт, поэтому его сбой страницу не перезагружает (WCAG 3.3.7).
 */
export function lazyPage<Module extends Record<string, unknown>, Name extends keyof Module>(
  importer: () => Promise<Module>,
  name: Name,
) {
  return lazyRouteComponent(
    () =>
      importer().catch((cause: unknown) => {
        throw isChunkLoadError(cause) ? new ChunkLoadError(cause) : cause;
      }),
    name,
  );
}
