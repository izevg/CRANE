import { useLingui } from "@lingui/react/macro";
import { useRouterState } from "@tanstack/react-router";
import { useEffect } from "react";
import { notFoundRoute } from "~/pages/not-found";
import { documentTitle, pageTitleOf } from "./title";

/**
 * Заголовок раздела (для крошек и title). Во время ожидания берётся заголовок целевой страницы, а не прежней.
 * Строки идут через каталог и пересчитываются при смене локали.
 */
export function usePageTitle(): string | undefined {
  const { i18n } = useLingui();
  const descriptor = useRouterState({
    select: (state) => pageTitleOf(state.matches, notFoundRoute.staticData.title),
  });
  return descriptor === undefined ? undefined : i18n._(descriptor);
}

/** Обновляет document.title; идёт раньше переноса фокуса, потому что фокус ставится после завершения навигации. */
export function useDocumentTitle(): void {
  const page = usePageTitle();
  useEffect(() => {
    document.title = documentTitle(page);
  }, [page]);
}
