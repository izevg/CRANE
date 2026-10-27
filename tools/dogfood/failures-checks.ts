// Чистые проверки сценариев отказов: вход — строки журнала crane, выход — список расхождений.
// Источник истины — сервер (access-журнал, журнал crane), а не вывод клиента.
import { type Access, type Egress, HTTP_OK_FROM, HTTP_OK_TO, SERVER_ERROR_FROM } from "./audit.ts";

const UNAVAILABLE = 503;
const SNIPPET_CHARS = 120;
const VALKEY_WARN = "Valkey:";
const EMPTY = "в окне нет ни одной строки access: без запросов проверка ничего не доказывает";

const describe = (line: Access): string => `${line.path}: status ${line.status}`;

/** Строки с недопустимым статусом; пустое окно — отказ: без запросов проверка ничего не доказывает. */
function unlessStatus(lines: Access[], bad: (status: number) => boolean): string[] {
  if (lines.length === 0) {
    return [EMPTY];
  }
  return lines.filter((line) => bad(line.status)).map(describe);
}

/** Ни одного 5xx: отказ Valkey не должен ронять proxy-чтения. */
export const checkNoServerErrors = (lines: Access[]): string[] =>
  unlessStatus(lines, (status) => status >= SERVER_ERROR_FROM);

/** Каждый ответ ровно 503: при отказе хранилища сервер говорит «повтори позже», а не 500 и не 502. */
export const checkUnavailable = (lines: Access[]): string[] =>
  unlessStatus(lines, (status) => status !== UNAVAILABLE);

/** Каждый ответ 2xx: после возврата сервиса запрос снова проходит. */
export const checkServed = (lines: Access[]): string[] =>
  unlessStatus(lines, (status) => status < HTTP_OK_FROM || status > HTTP_OK_TO);

/** В окне отказа нет ни одной выборки из upstream: при недоступном каталоге proxy не ходит наружу. */
export function checkNoEgress(egress: Egress[]): string[] {
  return egress.map((line) => `выборка из upstream: ${line.path}`);
}

/**
 * Холодный ключ без L2 и без координатора: один ответ 200 с `cache=miss` и ровно одна выборка
 * из upstream (`upstreamPath` — путь ключа у upstream).
 */
export function checkDegradedFetch(
  access: Access[],
  egress: Egress[],
  upstreamPath: string,
): string[] {
  const problems: string[] = [];
  if (access.length !== 1) {
    problems.push(`access-строк холодного ключа ${access.length}, ожидалась одна`);
  }
  for (const line of access) {
    if (line.status !== HTTP_OK_FROM || line.cache !== "miss") {
      problems.push(`ожидались 200 и cache=miss, получено ${line.status}/${line.cache ?? "-"}`);
    }
  }
  const fetches = egress.filter((line) => line.path === upstreamPath);
  if (fetches.length === 0) {
    problems.push(`нет выборки из upstream для ${upstreamPath}`);
  }
  return problems;
}

/** Нет паник и строк уровня ERROR. Паника печатается обычным текстом (не JSON), поэтому ищется по сырой строке. */
export function checkNoFaults(craneNdjson: string): string[] {
  const problems: string[] = [];
  for (const raw of craneNdjson.split("\n")) {
    if (raw.includes("panicked")) {
      problems.push(`паника: ${raw.slice(0, SNIPPET_CHARS)}`);
    }
    if (parseObject(raw)?.level === "ERROR") {
      problems.push(`уровень ERROR: ${raw.slice(0, SNIPPET_CHARS)}`);
    }
  }
  return problems;
}

/** Деградация кэша видна и безопасна: есть warn адаптера Valkey о потере соединения, нет паник и ERROR. */
export function checkCacheDegraded(craneNdjson: string): string[] {
  const warned = craneNdjson.split("\n").some((raw) => {
    const entry = parseObject(raw);
    return (
      entry?.level === "WARN" &&
      typeof entry.message === "string" &&
      entry.message.includes(VALKEY_WARN)
    );
  });
  const missing = warned ? [] : ["в журнале нет warn адаптера Valkey о потере соединения"];
  return [...checkNoFaults(craneNdjson), ...missing];
}

interface Entry {
  level?: unknown;
  message?: unknown;
}

function parseObject(raw: string): Entry | undefined {
  try {
    const value: unknown = JSON.parse(raw);
    return typeof value === "object" && value !== null && !Array.isArray(value)
      ? (value as Entry)
      : undefined;
  } catch {
    return undefined;
  }
}
