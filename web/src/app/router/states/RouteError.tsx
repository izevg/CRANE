import { useLingui } from "@lingui/react/macro";
import { type ErrorComponentProps, Navigate, useRouter } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { loginRoute } from "~/pages/login";
import { notFoundRoute } from "~/pages/not-found";
import { ApiProblem, problemViewOf } from "~/shared/api";
import { ChunkLoadError } from "~/shared/lib/route";
import type { Action } from "~/shared/ui/actions";
import { ProblemState, type ProblemView } from "~/shared/ui/status";
import { focusMainHeading, isFocusInMain } from "../focus/routeFocus";
import { type ChunkAction, decideChunkAction, markReload } from "../recovery/chunk";
import { reloadPage } from "../recovery/reload";
import { useAutoRetry } from "../recovery/useAutoRetry";

const NOT_FOUND = 404;

/** Решение по сбою чанка принимается один раз на показ ошибки; запись метки и перезагрузка — в эффекте. */
function useChunkAction(error: unknown): ChunkAction | undefined {
  const [action] = useState<ChunkAction | undefined>(() =>
    error instanceof ChunkLoadError ? decideChunkAction() : undefined,
  );
  useEffect(() => {
    if (action === "reload") {
      markReload();
      reloadPage();
    }
  }, [action]);
  return action;
}

type Actions = { retry: Action; home: Action; reload: Action };

function viewOf(
  error: unknown,
  chunk: ChunkAction | undefined,
  actions: Actions,
): ProblemView | null {
  if (chunk === "reload") {
    // страница вот-вот перезагрузится: ошибку не показываем
    return null;
  }
  if (chunk === "offline") {
    return { kind: "offline", cause: "network", retry: actions.retry };
  }
  if (chunk === "show") {
    return { kind: "client", cause: "chunk", reload: actions.reload };
  }
  if (error instanceof ApiProblem) {
    return problemViewOf(error, { retry: actions.retry, home: actions.home });
  }
  return { kind: "client", cause: "render", reload: actions.reload };
}

type Props = ErrorComponentProps & {
  /** Сбой до появления шелла: недоступна вся веб-консоль, а не раздел. */
  scope?: "section" | "console";
};

/**
 * Ошибка маршрута в области содержимого (шелл остаётся). Рисуется только при переходе: preload ошибку проглатывает,
 * опрос и мутации сюда не доходят, поэтому 401 здесь — это переход, и он уводит на вход. Тексты исключений и стек не
 * показываются. Сбой кода страницы (`ChunkLoadError`) ведёт к одной перезагрузке; сбой ленивого кода внутри страницы
 * (палитра, график) — обычная ошибка рендера без перезагрузки (WCAG 3.3.7). Недоступный сервер и сеть повторяются сами.
 */
export function RouteError({ error, reset, scope = "section" }: Props) {
  const { t } = useLingui();
  const router = useRouter();
  const chunk = useChunkAction(error);
  const retry = useCallback(() => {
    // Блок ошибки пересоздаётся, и фокус, стоявший в нём, ушёл бы на body (WCAG 2.4.3): после повтора он на h1
    // результата. Фокус вне main (сайдбар, topbar) не трогаем.
    const keepFocus = isFocusInMain();
    reset();
    const loading = router.invalidate();
    if (keepFocus) {
      const focus = (): void => {
        // макрозадача: React к этому моменту успевает заменить узлы результатом
        setTimeout(focusMainHeading, 0);
      };
      void loading.then(focus, focus);
    }
  }, [reset, router]);
  const problem = viewOf(error, chunk, {
    retry: { label: t`Повторить`, run: { onSelect: retry } },
    home: { label: t`На главную`, run: { href: "/" } },
    reload: { label: t`Обновить страницу`, run: { onSelect: reloadPage } },
  });
  useAutoRetry(
    problem?.kind === "offline",
    retry,
    error instanceof ApiProblem ? error.retryAfterMs : undefined,
  );
  if (error instanceof ApiProblem && error.kind === "unauthenticated") {
    return (
      <Navigate
        to="/login"
        replace
        search={loginRoute.validateSearch({
          reason: error.reason,
          redirect: router.latestLocation.href,
        })}
      />
    );
  }
  if (error instanceof ApiProblem && error.status === NOT_FOUND && error.kind === "problem") {
    // тот же экран, что у адреса без маршрута: по разметке скрытый ресурс от несуществующего не отличить
    return <notFoundRoute.component />;
  }
  if (problem?.kind === "planned" && scope === "console") {
    return <ProblemState headingLevel={1} problem={{ ...problem, scope }} />;
  }
  return problem === null ? null : <ProblemState headingLevel={1} problem={problem} />;
}
