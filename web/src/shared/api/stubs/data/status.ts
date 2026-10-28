import type { Status } from "../../generated/schemas";

/** Маркер модуля данных: шаг сборки ищет его в dist и валит production-сборку без демо-флага. */
export const STUB_MARKER = "crane-stub:status";

const FEEDS_LAG_S = 540;
const SIEM_LAG_S = 12;

// i18n-fixture: демо-данные заглушки — контент сервера, а не текст интерфейса
export function getStatus(): Status {
  return {
    version: "0.3.0",
    mode: "degraded",
    feeds_max_lag_s: FEEDS_LAG_S,
    siem_export_lag_s: SIEM_LAG_S,
    degraded: ["semantic", "upstream:npmjs"],
    _links: { self: { href: "/status" } },
    components: [
      { name: "PostgreSQL", kind: "database", state: "ok", latency_ms: 4 },
      { name: "Object storage", kind: "storage", state: "ok", latency_ms: 18 },
      { name: "Keycloak", kind: "identity", state: "ok", latency_ms: 31 },
      { name: "Cedar", kind: "authorization", state: "ok", latency_ms: 2 },
      {
        name: "Scanner worker",
        kind: "worker",
        state: "degraded",
        latency_ms: 940,
        since: "2026-10-28T08:14:00Z",
        detail: "Очередь сканирования растёт",
      },
      { name: "Upstream npmjs", kind: "upstream", state: "down", latency_ms: null },
    ],
  };
}
