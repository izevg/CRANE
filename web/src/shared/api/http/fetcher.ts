import { createCsrfSource } from "./csrf";
import { ApiProblem, networkProblem, problemFromResponse } from "./problem";

const API_PREFIX = "/api/v1";
const CSRF_HEADER = "X-CSRF-Token";
const AUTHORIZATION_HEADER = "Authorization";
const SESSION_URL = `${API_PREFIX}/session`;
const READ_METHODS = new Set(["GET", "HEAD"]);
/** Один повтор после перечитывания токена: второй CSRF-отказ подряд — уже не устаревший токен. */
const MAX_CSRF_RETRIES = 1;

export type Transport = {
  send: <T>(url: string, options: RequestInit) => Promise<T>;
};

type Deps = {
  fetch: typeof fetch;
  /** Токен из `GET /session`; `null` — сессии нет. */
  readCsrfToken: () => Promise<string | null>;
};

async function readBody(response: Response): Promise<unknown> {
  const raw = await response.text();
  try {
    return raw === "" ? undefined : JSON.parse(raw);
  } catch {
    // 2xx с телом не JSON (прокси подменил ответ): данных нет, это сбой, а не пустой успех
    throw new ApiProblem({ kind: "problem", status: response.status });
  }
}

async function readCsrfToken(request: typeof fetch): Promise<string | null> {
  const response = await request(SESSION_URL, { credentials: "same-origin", cache: "no-store" });
  if (!response.ok) {
    throw await problemFromResponse(response);
  }
  const body = await readBody(response);
  const token =
    typeof body === "object" && body !== null && "csrf_token" in body ? body.csrf_token : null;
  return typeof token === "string" ? token : null;
}

async function request(deps: Deps, url: string, init: RequestInit): Promise<Response> {
  try {
    return await deps.fetch(`${API_PREFIX}${url}`, init);
  } catch (error) {
    if (error instanceof DOMException && error.name === "AbortError") {
      throw error;
    }
    throw networkProblem();
  }
}

/**
 * Транспорт API: cookie той же страницы, сеть всегда ревалидируется (GET `no-cache`, мутации `no-store`), токен CSRF
 * на мутациях, ошибки — `ApiProblem`. If-None-Match и 304 делает HTTP-кэш браузера (`no-cache`), Idempotency-Key
 * транспорт не ставит: ключ принадлежит намерению пользователя и приходит параметром операции.
 */
export function createTransport(deps: Deps): Transport {
  const csrf = createCsrfSource(deps.readCsrfToken);

  async function attempt<T>(url: string, options: RequestInit, retriesLeft: number): Promise<T> {
    const read = READ_METHODS.has((options.method ?? "GET").toUpperCase());
    const headers = new Headers(options.headers);
    // Запрос с Bearer (мастер онбординга, CLI) аутентифицируется только заголовком: cookie не отправляем, чтобы у
    // сервера не было двух кредитивов, а CSRF не нужен. Пустой Authorization — не Bearer, это обычный запрос.
    const bearer = (headers.get(AUTHORIZATION_HEADER) ?? "") !== "";
    const needsCsrf = !(read || bearer);
    if (needsCsrf) {
      headers.set(CSRF_HEADER, await csrf.token());
    }
    const response = await request(deps, url, {
      ...options,
      headers,
      credentials: bearer ? "omit" : "same-origin",
      cache: read ? "no-cache" : "no-store",
    });
    if (response.ok) {
      return {
        data: await readBody(response),
        status: response.status,
        headers: response.headers,
      } as T;
    }
    const problem = await problemFromResponse(response);
    if (problem.kind === "csrf" && needsCsrf && retriesLeft > 0) {
      csrf.reset();
      return attempt<T>(url, options, retriesLeft - 1);
    }
    throw problem;
  }

  return { send: (url, options) => attempt(url, options, MAX_CSRF_RETRIES) };
}

/** Боевой транспорт: глобальный `fetch` (подменяется в тестах на уровне окружения) и токен из `GET /session`. */
export function createDefaultTransport(): Transport {
  const request: typeof fetch = (input, init) => fetch(input, init);
  return createTransport({ fetch: request, readCsrfToken: () => readCsrfToken(request) });
}

const transport = createDefaultTransport();

/** Тип ошибки для сгенерированных опций (orval читает это имя): всё, что бросают транспорт и заглушки. */
export type ErrorType<_Error> = ApiProblem;

/** Мутатор orval: единственный транспорт; тип ответа задаёт сгенерированный код. */
export async function fetcher<T>(url: string, options: RequestInit): Promise<T> {
  return transport.send<T>(url, options);
}
