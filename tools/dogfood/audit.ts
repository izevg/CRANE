// Проверка журнала crane после сценария: события аудита (OCSF, плоские ключи) и утечки секретов.
// Чистые функции: вход — текст журнала и ожидания, выход — список расхождений.

export type Operation = "publish" | "yank" | "unyank";

export type Expected =
  | { kind: "api"; agent: string; operation: Operation; purl: string }
  /** Не меньше одного отказа аутентификации от этого клиента (число попыток cargo не детерминировано). */
  | { kind: "authFailure"; agent: string };

export interface Mismatch {
  check: string;
  detail: string;
}

export interface Options {
  /** Имя принципала токена: ожидаемый `actor.user.uid` во всех событиях 6003. */
  principal: string;
  /** Адреса, с которых клиент ходил в Caddy: `src_endpoint.ip` обязан быть одним из них. */
  loopback: string[];
}

export interface Secret {
  name: string;
  value: string;
}

/** Строка журнала. Ключи OCSF плоские: `actor.user.uid` — имя поля, а не вложенный объект. */
interface Line {
  target?: unknown;
  class_uid?: unknown;
  status?: unknown;
  method?: unknown;
  path?: unknown;
  request_id?: unknown;
  "api.operation"?: unknown;
  "resources.uid"?: unknown;
  "actor.user.uid"?: unknown;
  "user.uid"?: unknown;
  "src_endpoint.ip"?: unknown;
  "metadata.correlation_uid"?: unknown;
  "http_request.user_agent"?: unknown;
  cache?: unknown;
  upstream_failure?: unknown;
  failure?: unknown;
}

/** Строка `crane::access`: откуда пришёл ответ proxy (`cache`) и почему upstream отказал. */
export interface Access {
  path: string;
  status: number;
  cache: string | undefined;
  failure: string | undefined;
}

/** Строка `crane::upstream`: одна реальная выборка из upstream (или отказ), по пути без query. */
export interface Egress {
  path: string;
  status: number | undefined;
  failure: string | undefined;
}

const HTTP_OK = 200;
const API_ACTIVITY = 6003;
const AUTHENTICATION = 3002;
const HTTP_OK_FROM = 200;
const HTTP_OK_TO = 299;
const SNIPPET_CHARS = 120;
const ROUTES: Record<Operation, { method: string; path: RegExp }> = {
  publish: { method: "PUT", path: /\/api\/v1\/crates\/new$/ },
  yank: { method: "DELETE", path: /\/api\/v1\/crates\/[^/]+\/[^/]+\/yank$/ },
  unyank: { method: "PUT", path: /\/api\/v1\/crates\/[^/]+\/[^/]+\/unyank$/ },
};

const text = (value: unknown): string | undefined =>
  typeof value === "string" ? value : undefined;

const isLine = (value: unknown): value is Line =>
  typeof value === "object" && value !== null && !Array.isArray(value);

function parse(ndjson: string, mismatches: Mismatch[]): Line[] {
  const lines: Line[] = [];
  for (const raw of ndjson.split("\n")) {
    if (raw.trim() === "") {
      continue;
    }
    let value: unknown;
    try {
      value = JSON.parse(raw);
    } catch {
      value = undefined;
    }
    if (isLine(value)) {
      lines.push(value);
    } else {
      mismatches.push({ check: "json", detail: `не JSON-объект: ${raw.slice(0, SNIPPET_CHARS)}` });
    }
  }
  return lines;
}

function counts(keys: string[]): Map<string, number> {
  const map = new Map<string, number>();
  for (const key of keys) {
    map.set(key, (map.get(key) ?? 0) + 1);
  }
  return map;
}

/** Мультимножества должны совпасть точно: лишнее событие такая же ошибка, как пропавшее. */
function diff(check: string, expected: string[], actual: string[]): Mismatch[] {
  const want = counts(expected);
  const have = counts(actual);
  const mismatches: Mismatch[] = [];
  for (const key of new Set([...want.keys(), ...have.keys()])) {
    const w = want.get(key) ?? 0;
    const h = have.get(key) ?? 0;
    if (w !== h) {
      mismatches.push({ check, detail: `${key}: ожидалось ${w}, в журнале ${h}` });
    }
  }
  return mismatches;
}

const userAgent = (line: Line): string => text(line["http_request.user_agent"]) ?? "";

function eventKey(line: Line): string {
  const operation = text(line["api.operation"]) ?? "?";
  const purl = text(line["resources.uid"]) ?? "?";
  return `${operation} ${purl} ${text(line.status) ?? "?"}`;
}

function checkApiEvent(line: Line, access: Map<string, Line>, options: Options): Mismatch[] {
  const label = eventKey(line);
  const found: Mismatch[] = [];
  const fail = (check: string, detail: string): void => {
    found.push({ check, detail: `${label}: ${detail}` });
  };

  if (line["actor.user.uid"] !== options.principal) {
    fail(
      "principal",
      `actor.user.uid=${String(line["actor.user.uid"])}, ожидался ${options.principal}`,
    );
  }
  const ip = text(line["src_endpoint.ip"]);
  if (ip === undefined || !options.loopback.includes(ip)) {
    fail(
      "src_endpoint",
      `src_endpoint.ip=${String(ip)}, ожидался ${options.loopback.join(" или ")}`,
    );
  }
  const correlation = text(line["metadata.correlation_uid"]);
  const entry = correlation === undefined ? undefined : access.get(correlation);
  if (entry === undefined) {
    fail("correlation", `нет строки crane::access для correlation_uid=${String(correlation)}`);
    return found;
  }
  const code = entry.status;
  if (typeof code !== "number" || code < HTTP_OK_FROM || code > HTTP_OK_TO) {
    fail("access", `статус запроса ${String(code)}, ожидался 2xx`);
  }
  const operation = text(line["api.operation"]);
  const route = ROUTES[operation as Operation];
  const path = text(entry.path) ?? "";
  if (route === undefined || entry.method !== route.method || !route.path.test(path)) {
    fail("access", `запрос ${String(entry.method)} ${path} не соответствует операции`);
  }
  return found;
}

function checkAuthFailures(agent: string, failures: Line[], required: boolean): Mismatch[] {
  const check = `3002 ${agent}`;
  const found: Mismatch[] = [];
  if (required && failures.length === 0) {
    found.push({
      check,
      detail: "ожидался хотя бы один отказ аутентификации, в журнале ни одного",
    });
  }
  for (const line of failures) {
    if (line.status !== "Failure") {
      found.push({ check, detail: `статус ${String(line.status)}, ожидался Failure` });
    }
    if (line["user.uid"] !== undefined) {
      found.push({ check, detail: "у отказа не должно быть user.uid" });
    }
  }
  return found;
}

/** Строки access-журнала по порядку; строки без пути или статуса пропускаются. */
export function accessLines(craneNdjson: string): Access[] {
  return parse(craneNdjson, []).flatMap((line) => {
    const path = text(line.path);
    if (line.target !== "crane::access" || path === undefined || typeof line.status !== "number") {
      return [];
    }
    return [
      { path, status: line.status, cache: text(line.cache), failure: text(line.upstream_failure) },
    ];
  });
}

/** Строки egress-журнала `crane::upstream` по порядку. */
export function egressLines(craneNdjson: string): Egress[] {
  return parse(craneNdjson, []).flatMap((line) => {
    const path = text(line.path);
    if (line.target !== "crane::upstream" || path === undefined) {
      return [];
    }
    const status = typeof line.status === "number" ? line.status : undefined;
    return [{ path, status, failure: text(line.failure) }];
  });
}

export interface Collapse {
  /** Путь запроса к crane (в access-журнале). */
  accessPath: string;
  /** Путь того же ключа у upstream (в egress-журнале). */
  upstreamPath: string;
  requests: number;
}

/**
 * Схлопывание холодного ключа: все запросы получили 200 с `cache=miss` (ведомые полёта видят тот же Miss,
 * что и ведущий; опоздавший получил бы hit, то есть без перекрытия условие не выполнится), а в egress-журнале
 * ровно одна выборка этого пути. Пустой результат — сошлось.
 */
export function checkCollapse(access: Access[], egress: Egress[], expected: Collapse): string[] {
  const mine = access.filter((line) => line.path === expected.accessPath);
  const fetches = egress.filter((line) => line.path === expected.upstreamPath);
  const problems: string[] = [];
  if (mine.length !== expected.requests) {
    problems.push(`access-строк ${mine.length}, ожидалось ${expected.requests}`);
  }
  const wrong = mine.filter((line) => line.status !== HTTP_OK || line.cache !== "miss");
  if (wrong.length > 0) {
    const seen = wrong.map((line) => `${line.status}/${line.cache ?? "-"}`).join(", ");
    problems.push(`ожидались 200 и cache=miss, получено: ${seen}`);
  }
  if (fetches.length !== 1) {
    problems.push(`выборок из upstream ${fetches.length}, ожидалась ровно одна`);
  }
  return problems;
}

const SERVED_FROM_CACHE = new Set(["hit", "stale", "stale-on-error"]);
const SERVER_ERROR_FROM = 500;

// config.json индекса crane собирает сам, upstream и кэш в нём не участвуют: поля `cache` у него нет.
const isConfig = (line: Access): boolean => line.path.endsWith("/index/config.json");

/**
 * Офлайн-фаза: строки access без единого промаха и без 5xx, все ответы из кэша (config.json индекса
 * проверяется только на 5xx). Пустая фаза — отказ: сборка без запросов ничего не доказывает.
 */
export function checkOffline(lines: Access[]): string[] {
  if (lines.every(isConfig)) {
    return ["в офлайн-фазе нет ни одного ответа proxy из кэша"];
  }
  return lines
    .filter(
      (line) =>
        line.status >= SERVER_ERROR_FROM ||
        (!isConfig(line) && (line.cache === undefined || !SERVED_FROM_CACHE.has(line.cache))),
    )
    .map((line) => `${line.path}: status ${line.status}, cache ${line.cache ?? "-"}`);
}

const BAD_GATEWAY = 502;

/**
 * Улика недоступности upstream. Ответы из кэша в upstream не ходят, поэтому сами по себе её не дают.
 * Нужны оба факта: проба холодного ключа получила 502 с `upstream_failure=unavailable`, а в egress-журнале
 * с отметки офлайна нет ни одной успешной выборки (строки со `status`) и есть хотя бы одна неудачная.
 * `probe` и `egress` взяты после отметки. Пустой результат — сошлось.
 */
export function checkUpstreamDown(probe: Access[], egress: Egress[]): string[] {
  const problems: string[] = [];
  if (probe.length !== 1) {
    problems.push(`access-строк пробы ${probe.length}, ожидалась одна`);
  }
  for (const line of probe) {
    if (line.status !== BAD_GATEWAY || line.failure !== "unavailable") {
      problems.push(
        `проба: ожидались 502 и upstream_failure=unavailable, получено ${line.status}/${line.failure ?? "-"}`,
      );
    }
  }
  const succeeded = egress.filter((line) => line.status !== undefined);
  if (succeeded.length > 0) {
    problems.push(`успешных выборок из upstream после отключения: ${succeeded.length}`);
  }
  if (!egress.some((line) => line.failure !== undefined)) {
    problems.push("в egress-журнале нет ни одной неудачной выборки");
  }
  return problems;
}

/** Распределение значений `cache` для лога; строки без значения считаются как «-». */
export function cacheDistribution(lines: Access[]): Record<string, number> {
  return Object.fromEntries(counts(lines.map((line) => line.cache ?? "-")));
}

/** Сверяет журнал crane (NDJSON) с ожиданиями сценария. Пустой результат — всё сошлось. */
export function verify(craneNdjson: string, expected: Expected[], options: Options): Mismatch[] {
  const mismatches: Mismatch[] = [];
  const lines = parse(craneNdjson, mismatches);
  const access = new Map<string, Line>();
  for (const line of lines) {
    const id = text(line.request_id);
    if (line.target === "crane::access" && id !== undefined) {
      access.set(id, line);
    }
  }
  const api = lines.filter((line) => line.class_uid === API_ACTIVITY);
  const auth = lines.filter((line) => line.class_uid === AUTHENTICATION);
  const agents = [...new Set(expected.map((e) => e.agent))];
  const ownedBy = (agent: string) => (line: Line) => userAgent(line).startsWith(agent);

  for (const line of [...api, ...auth]) {
    if (!agents.some((agent) => ownedBy(agent)(line))) {
      mismatches.push({
        check: "agent",
        detail: `событие ${String(line.class_uid)} от неизвестного клиента: ${userAgent(line)}`,
      });
    }
  }
  for (const agent of agents) {
    const wanted = expected.flatMap((e) =>
      e.kind === "api" && e.agent === agent ? [`${e.operation} ${e.purl} Success`] : [],
    );
    const mine = api.filter(ownedBy(agent));
    mismatches.push(...diff(`6003 ${agent}`, wanted, mine.map(eventKey)));
    for (const line of mine) {
      mismatches.push(...checkApiEvent(line, access, options));
    }
    const required = expected.some((e) => e.kind === "authFailure" && e.agent === agent);
    mismatches.push(...checkAuthFailures(agent, auth.filter(ownedBy(agent)), required));
  }
  return mismatches;
}

/** Имена секретов, значения которых встретились в сыром журнале. Сами значения не возвращаются. */
export function leaks(rawLogs: string, secrets: Secret[]): string[] {
  return secrets
    .filter((secret) => secret.value !== "" && rawLogs.includes(secret.value))
    .map((secret) => secret.name);
}
