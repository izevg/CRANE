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
}

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
