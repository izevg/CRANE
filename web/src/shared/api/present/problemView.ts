import { msg } from "@lingui/core/macro";
import { i18n } from "~/shared/lib/i18n";
import type { Action } from "~/shared/ui/actions";
import type { ProblemView, SessionReason } from "~/shared/ui/status";
import type { ApiProblem } from "../http/problem";

/** Действия строит вызывающий (подписи и обработчики принадлежат экрану); вид только решает, какие из них уместны. */
export type ProblemActions = {
  retry?: Action;
  home?: Action;
  signIn?: Action;
};

const STATUS_NOT_FOUND = 404;
const STATUS_FORBIDDEN = 403;
const STATUS_BAD_GATEWAY = 502;
const STATUS_UNAVAILABLE = 503;
const STATUS_GATEWAY_TIMEOUT = 504;
const SERVER_DOWN_STATUSES: ReadonlySet<number> = new Set([
  STATUS_BAD_GATEWAY,
  STATUS_UNAVAILABLE,
  STATUS_GATEWAY_TIMEOUT,
]);
const SESSION_REASONS: readonly string[] = [
  "session_expired",
  "session_revoked",
  "session_elsewhere",
];

function isSessionReason(value: string | undefined): value is SessionReason {
  return value !== undefined && SESSION_REASONS.includes(value);
}

function offlineOf(problem: ApiProblem, actions: ProblemActions): ProblemView {
  // Нет сети у пользователя и недоступный сервер лечатся по-разному, поэтому причину различаем.
  const cause = problem.kind === "network" && !navigator.onLine ? "network" : "server";
  return { kind: "offline", cause, ...(actions.retry ? { retry: actions.retry } : {}) };
}

function errorOf(problem: ApiProblem, actions: ProblemActions): ProblemView {
  return {
    kind: "error",
    title: problem.title === "" ? i18n._(msg`Не удалось выполнить запрос`) : problem.title,
    remediation: problem.remediation,
    ...(problem.detail === undefined ? {} : { detail: problem.detail }),
    ...(problem.ruleId === undefined ? {} : { ruleId: problem.ruleId }),
    ...(problem.requestId === undefined ? {} : { requestId: problem.requestId }),
    ...(actions.retry ? { retry: actions.retry } : {}),
  };
}

function forbiddenOf(problem: ApiProblem, actions: ProblemActions): ProblemView | undefined {
  // 403 без правила или идентификатора запроса не показывается как «отказ по правилу»: нечего назвать.
  if (problem.ruleId === undefined || problem.requestId === undefined) {
    return undefined;
  }
  return {
    kind: "forbidden",
    ruleId: problem.ruleId,
    requestId: problem.requestId,
    remediation: problem.remediation,
    ...(problem.contact === undefined ? {} : { contact: problem.contact }),
    ...(actions.home ? { home: actions.home } : {}),
  };
}

function httpProblemView(problem: ApiProblem, actions: ProblemActions): ProblemView {
  if (problem.status === STATUS_NOT_FOUND && actions.home) {
    return {
      kind: "not-found",
      home: actions.home,
      ...(problem.requestId === undefined ? {} : { requestId: problem.requestId }),
    };
  }
  if (problem.status === STATUS_FORBIDDEN) {
    const forbidden = forbiddenOf(problem, actions);
    if (forbidden) {
      return forbidden;
    }
  }
  if (SERVER_DOWN_STATUSES.has(problem.status)) {
    return offlineOf(problem, actions);
  }
  return errorOf(problem, actions);
}

/**
 * Вид проблемы для `ProblemState`: разбор problem+json живёт здесь, а не в маршрутах и страницах. Ошибка, которая
 * не `ApiProblem`, сюда не попадает: это сбой интерфейса, и вид для него (`client`) выбирает вызывающий.
 */
export function problemViewOf(problem: ApiProblem, actions: ProblemActions = {}): ProblemView {
  switch (problem.kind) {
    case "planned":
      return { kind: "planned", replacedIn: problem.replacedIn ?? "" };
    case "unauthenticated":
      return {
        kind: "session",
        ...(isSessionReason(problem.reason) ? { reason: problem.reason } : {}),
        ...(actions.signIn ? { signIn: actions.signIn } : {}),
      };
    case "network":
      return offlineOf(problem, actions);
    case "step-up":
    case "csrf":
    case "problem":
      return httpProblemView(problem, actions);
  }
}
