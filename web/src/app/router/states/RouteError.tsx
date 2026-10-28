import { useLingui } from "@lingui/react/macro";
import { type ErrorComponentProps, useRouter } from "@tanstack/react-router";
import { useCallback, useEffect, useState } from "react";
import { ApiProblem, problemViewOf } from "~/shared/api";
import type { Action } from "~/shared/ui/actions";
import { ProblemState, type ProblemView } from "~/shared/ui/status";
import { type ChunkAction, ChunkLoadError, decideChunkAction, markReload } from "../chunk/chunk";
import { reloadPage } from "../chunk/reload";
import { NotFoundPage } from "./notFound";

const NOT_FOUND = 404;

function useChunkAction(error: unknown): ChunkAction | undefined {
  const chunk = error instanceof ChunkLoadError;
  // Решение принимается один раз на показ ошибки; запись метки и перезагрузка — в эффекте.
  const [action] = useState<ChunkAction | undefined>(() =>
    chunk ? decideChunkAction() : undefined,
  );
  useEffect(() => {
    if (chunk && decideChunkAction() === "reload") {
      markReload();
      reloadPage();
    }
  }, [chunk]);
  return action;
}

type Actions = { retry: Action; home: Action; reload: Action };

function clientView(
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

/**
 * Ошибка маршрута в области содержимого (шелл остаётся). Тексты исключений и стек не показываются. Сбой кода
 * самой страницы (`ChunkLoadError` из `lazyPage`) ведёт к одной перезагрузке; сбой ленивого кода внутри страницы
 * (палитра, график) — обычная ошибка рендера без перезагрузки (WCAG 3.3.7).
 */
export function RouteError({ error, reset }: ErrorComponentProps) {
  const { t } = useLingui();
  const router = useRouter();
  const chunk = useChunkAction(error);
  const retry = useCallback(() => {
    reset();
    void router.invalidate();
  }, [reset, router]);
  if (error instanceof ApiProblem && error.status === NOT_FOUND && error.kind === "problem") {
    // тот же экран, что у адреса без маршрута: по разметке скрытый ресурс от несуществующего не отличить
    return <NotFoundPage />;
  }
  const problem = clientView(error, chunk, {
    retry: { label: t`Повторить`, run: { onSelect: retry } },
    home: { label: t`На главную`, run: { href: "/" } },
    reload: { label: t`Обновить страницу`, run: { onSelect: reloadPage } },
  });
  return problem === null ? null : <ProblemState headingLevel={1} problem={problem} />;
}
