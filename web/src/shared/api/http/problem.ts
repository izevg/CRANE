/**
 * Единый класс ошибки API. Разбор `application/problem+json` делается здесь один раз: экраны видят только `kind`
 * и готовые поля, а не сырое тело.
 */
export type ApiProblemKind =
  /** 401 сессии без `required_acr`: нужен вход. */
  | "unauthenticated"
  /** 401 с `required_acr`: нужен step-up, а не вход. */
  | "step-up"
  /** 403 из-за токена CSRF: транспорт перечитывает токен и повторяет запрос один раз. */
  | "csrf"
  /** Любой другой ответ problem+json (в том числе не-JSON тело). */
  | "problem"
  /** Запрос не дошёл до сервера (нет сети, сервер недоступен). Статуса нет. */
  | "network"
  /** Операция ещё заглушена: данных нет, раздел появится в `replacedIn`. Сеть не использовалась. */
  | "planned";

export type ApiProblemInit = {
  kind: ApiProblemKind;
  /** HTTP-статус; 0 у ошибок, созданных на клиенте (`network`, `planned`). */
  status: number;
  /** Текст сервера как пришёл; пустой, если тела нет. */
  title?: string | undefined;
  detail?: string | undefined;
  reason?: string | undefined;
  ruleId?: string | undefined;
  requestId?: string | undefined;
  remediation?: readonly string[] | undefined;
  contact?: string | undefined;
  requiredAcr?: string | undefined;
  retryAfterMs?: number | undefined;
  replacedIn?: string | undefined;
};

export class ApiProblem extends Error {
  readonly kind: ApiProblemKind;
  readonly status: number;
  readonly title: string;
  readonly detail: string | undefined;
  readonly reason: string | undefined;
  readonly ruleId: string | undefined;
  readonly requestId: string | undefined;
  readonly remediation: readonly string[];
  readonly contact: string | undefined;
  readonly requiredAcr: string | undefined;
  readonly retryAfterMs: number | undefined;
  readonly replacedIn: string | undefined;

  constructor(init: ApiProblemInit) {
    super(init.title ?? init.kind);
    this.name = "ApiProblem";
    this.kind = init.kind;
    this.status = init.status;
    this.title = init.title ?? "";
    this.detail = init.detail;
    this.reason = init.reason;
    this.ruleId = init.ruleId;
    this.requestId = init.requestId;
    this.remediation = init.remediation ?? [];
    this.contact = init.contact;
    this.requiredAcr = init.requiredAcr;
    this.retryAfterMs = init.retryAfterMs;
    this.replacedIn = init.replacedIn;
  }
}

const STATUS_UNAUTHORIZED = 401;
const STATUS_FORBIDDEN = 403;
const MS_PER_SECOND = 1000;
const CSRF_PROBLEM = /(^|\/)csrf([_-]|$)/;
const DIGITS = /^\d+$/;

type Body = Record<string, unknown>;

function isBody(value: unknown): value is Body {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function text(body: Body, key: string): string | undefined {
  const value = body[key];
  return typeof value === "string" && value !== "" ? value : undefined;
}

function strings(body: Body, key: string): readonly string[] {
  const value = body[key];
  return Array.isArray(value)
    ? value.filter((item): item is string => typeof item === "string")
    : [];
}

function contactOf(body: Body): string | undefined {
  const { contact } = body;
  return isBody(contact) ? text(contact, "display") : undefined;
}

/**
 * `Retry-After`: секунды или дата HTTP. Нечитаемое значение даёт `undefined`, а не 0: «повторить сразу» — это
 * решение сервера, которое нельзя выдумать из мусора.
 */
export function parseRetryAfter(header: string | null, now: number): number | undefined {
  if (header === null) {
    return undefined;
  }
  const value = header.trim();
  if (DIGITS.test(value)) {
    return Number(value) * MS_PER_SECOND;
  }
  const date = Date.parse(value);
  return Number.isNaN(date) ? undefined : Math.max(date - now, 0);
}

function kindOf(status: number, body: Body): ApiProblemKind {
  if (status === STATUS_UNAUTHORIZED) {
    return text(body, "required_acr") === undefined ? "unauthenticated" : "step-up";
  }
  // Контракт не описывает признак ошибки CSRF: ждём `type` вида `…/problems/csrf` или `reason` с префиксом `csrf`.
  const marker = `${text(body, "type") ?? ""} ${text(body, "reason") ?? ""}`;
  const csrf =
    status === STATUS_FORBIDDEN && marker.split(" ").some((part) => CSRF_PROBLEM.test(part));
  return csrf ? "csrf" : "problem";
}

/** Разбирает уже прочитанное тело ответа. Не JSON-объект (HTML прокси, пустое тело) даёт `problem` без полей. */
export function parseProblem(
  status: number,
  body: unknown,
  extra: { requestId?: string | null; retryAfter?: string | null; now: number },
): ApiProblem {
  const fields = isBody(body) ? body : {};
  return new ApiProblem({
    kind: kindOf(status, fields),
    status,
    title: text(fields, "title"),
    detail: text(fields, "detail"),
    reason: text(fields, "reason"),
    ruleId: text(fields, "rule_id"),
    requestId: text(fields, "request_id") ?? extra.requestId ?? undefined,
    remediation: strings(fields, "remediation"),
    contact: contactOf(fields),
    requiredAcr: text(fields, "required_acr"),
    retryAfterMs: parseRetryAfter(extra.retryAfter ?? null, extra.now),
  });
}

export async function problemFromResponse(response: Response): Promise<ApiProblem> {
  let body: unknown;
  try {
    body = JSON.parse(await response.text());
  } catch {
    // тело не JSON (страница прокси, пустой ответ): остаётся только статус
    body = undefined;
  }
  return parseProblem(response.status, body, {
    requestId: response.headers.get("X-Request-Id"),
    retryAfter: response.headers.get("Retry-After"),
    now: Date.now(),
  });
}

export function networkProblem(): ApiProblem {
  return new ApiProblem({ kind: "network", status: 0 });
}

export function plannedProblem(replacedIn: string): ApiProblem {
  return new ApiProblem({ kind: "planned", status: 0, replacedIn });
}
