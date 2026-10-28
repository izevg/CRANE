// Аудит защиты внутренних имён: события `namespace.reserve` и `upstream.shadow` (OCSF 6003) без ресурса.
// Чистые функции: разбор ожиданий и проверка резервирований из конфигурации сервера.
import type { Expected, Mismatch } from "./audit.ts";

/** Первая публикация нового имени резервирует его: отдельное событие без ресурса. */
export interface ReserveExpected {
  kind: "reserve";
  agent: string;
}

/** Отказ proxy отдавать внутреннее имя: событие без ресурса, запрос получил 404. */
export interface ShadowExpected {
  kind: "shadow";
  agent: string;
}

export type NamespaceExpected = ReserveExpected | ShadowExpected;

/** Актор резервирований, применённых из конфигурации при старте: у них нет запроса и клиента. */
export const CONFIG_ACTOR = "crane:config";
export const RESERVE = "namespace.reserve";
export const SHADOW = "upstream.shadow";

interface Route {
  method: string;
  path: RegExp;
}

const NPM_PATH = /^\/npm\/[^/]+\//;

/** Маршруты запросов, порождающих события: резервирование рождается публикацией, отказ — чтением. */
export const NAMESPACE_ROUTES: Record<string, Route[]> = {
  [RESERVE]: [
    { method: "PUT", path: /\/api\/v1\/crates\/new$/ },
    { method: "PUT", path: NPM_PATH },
  ],
  [SHADOW]: [
    { method: "GET", path: /\/(index\/.+|api\/v1\/crates\/[^/]+\/[^/]+\/download)$/ },
    { method: "GET", path: NPM_PATH },
  ],
};

/** Ключи ожидаемых событий в форме `eventKey` (purl без ресурса — «?»); `authFailure` событий 6003 не даёт. */
export function expectedKeys(expected: Expected): string[] {
  switch (expected.kind) {
    case "api":
      return [`${expected.operation} ${expected.purl} ${expected.outcome ?? "Success"}`];
    case "reserve":
      return [`${RESERVE} ? Success`];
    case "shadow":
      return [`${SHADOW} ? Failure`];
    default:
      return [];
  }
}

interface Event {
  "actor.user.uid"?: unknown;
  "api.operation"?: unknown;
  status?: unknown;
}

/** Делит события 6003 на резервирования конфигурации и клиентские. */
export function splitConfigEvents<T extends Event>(events: T[]): { byConfig: T[]; byClient: T[] } {
  return {
    byConfig: events.filter((line) => line["actor.user.uid"] === CONFIG_ACTOR),
    byClient: events.filter((line) => line["actor.user.uid"] !== CONFIG_ACTOR),
  };
}

/** Резервирования конфигурации: ровно ожидаемое число, все успешные. */
export function checkConfigReservations(events: Event[], expected: number): Mismatch[] {
  const check = "config reservations";
  const found: Mismatch[] = [];
  if (events.length !== expected) {
    found.push({ check, detail: `событий ${events.length}, ожидалось ${expected}` });
  }
  for (const line of events) {
    if (line["api.operation"] !== RESERVE || line.status !== "Success") {
      found.push({
        check,
        detail: `${String(line["api.operation"])} ${String(line.status)}: ожидалось ${RESERVE} Success`,
      });
    }
  }
  return found;
}

/** Снимает резервирования конфигурации с событий 6003, сверяет их число и возвращает клиентские события. */
export function takeConfigEvents<T extends Event>(
  events: T[],
  expected: number,
  mismatches: Mismatch[],
): T[] {
  const { byConfig, byClient } = splitConfigEvents(events);
  mismatches.push(...checkConfigReservations(byConfig, expected));
  return byClient;
}
