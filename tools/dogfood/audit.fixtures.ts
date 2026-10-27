import type { Expected, Mismatch } from "./audit.ts";

export const AGENT = "cargo/1.99.0";
export const PRINCIPAL = "dogfood-1";
export const OPTIONS = { principal: PRINCIPAL, loopback: ["127.0.0.1", "::1"] };
export const A_010 = "pkg:cargo/dfa_1_99@0.1.0";
export const A_011 = "pkg:cargo/dfa_1_99@0.1.1";

export interface Overrides {
  uid?: string;
  ip?: string;
  agent?: string;
  correlation?: string;
  status?: string;
}

export const ROUTE = {
  publish: { method: "PUT", path: "/cargo/internal/api/v1/crates/new" },
  yank: { method: "DELETE", path: "/cargo/internal/api/v1/crates/dfa_1_99/0.1.1/yank" },
  unyank: { method: "PUT", path: "/cargo/internal/api/v1/crates/dfa_1_99/0.1.1/unyank" },
} as const;

/** Пара строк журнала: событие 6003 и его access-строка. */
export function api(
  operation: keyof typeof ROUTE,
  purl: string,
  id: string,
  overrides: Overrides = {},
): string[] {
  const agent = overrides.agent ?? `${AGENT} (abc)`;
  const audit = {
    target: "crane::audit",
    class_uid: 6003,
    "api.operation": operation,
    status: overrides.status ?? "Success",
    "resources.uid": purl,
    "actor.user.uid": overrides.uid ?? PRINCIPAL,
    "src_endpoint.ip": overrides.ip ?? "127.0.0.1",
    "metadata.correlation_uid": overrides.correlation ?? id,
    "http_request.user_agent": agent,
  };
  const access = {
    target: "crane::access",
    request_id: id,
    status: 200,
    ...ROUTE[operation],
    user_agent: agent,
  };
  return [JSON.stringify(audit), JSON.stringify(access)];
}

export const authFailure = (agent = `${AGENT} (abc)`): string =>
  JSON.stringify({
    target: "crane::audit",
    class_uid: 3002,
    status: "Failure",
    "http_request.user_agent": agent,
  });

export const API_EXPECTED: Expected[] = [
  { kind: "api", agent: AGENT, operation: "publish", purl: A_010 },
  { kind: "api", agent: AGENT, operation: "yank", purl: A_011 },
  { kind: "authFailure", agent: AGENT },
];

export const goodLog = (): string[] => [
  ...api("publish", A_010, "r1"),
  ...api("yank", A_011, "r2"),
  authFailure(),
];

export const checks = (mismatches: Mismatch[]): string[] => mismatches.map((m) => m.check);
