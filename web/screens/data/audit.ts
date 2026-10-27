// biome-ignore-all lint/style/noMagicNumbers: детерминированные данные макетов, не продуктовый код
type ActorKind = "Человек" | "Робот" | "Workload" | "Агент";
export type Decision = "allow" | "deny" | "challenge";
type Channel = "UI" | "CLI" | "API" | "CI" | "A2A" | "SOAR";

export type AuditEvent = {
  id: string;
  /** UTC с миллисекундами. */
  time: string;
  actor: { alias: string; kind: ActorKind };
  action: { code: string; label: string };
  resource: string;
  decision: Decision;
  rule: string;
  channel: Channel;
  requestId: string;
};

const ACTIONS = [
  { code: "package.version.blocked", label: "Версия заблокирована" },
  { code: "package.version.published", label: "Версия опубликована" },
  { code: "token.issued", label: "Токен выпущен" },
  { code: "policy.cedar.updated", label: "Политика изменена" },
  { code: "audit.viewed", label: "Журнал просмотрен" },
  { code: "finding.vex.accepted", label: "VEX принят" },
  { code: "repository.retention.applied", label: "Retention применён" },
  { code: "audit.revealed", label: "Данные раскрыты" },
] as const;

const ACTORS = [
  { alias: "user-7f3a", kind: "Человек" },
  { alias: "user-c912", kind: "Человек" },
  { alias: "robot-ui-release", kind: "Робот" },
  { alias: "wl-ingest-eu1", kind: "Workload" },
  { alias: "agent-triage-02", kind: "Агент" },
  { alias: "user-04be", kind: "Человек" },
] as const satisfies readonly { alias: string; kind: ActorKind }[];

const RESOURCES = [
  "@acme/ui@1.4.2",
  "lodash@4.17.20",
  "npm-hosted",
  "policy/trust-default",
  "токен crp_9f8e7d6c",
  "payments/ledger-core@3.8.1",
  "pypi-hosted/requests@2.32.3",
  "findings/F-20418",
] as const;

const RULES = [
  "cedar:block-kev",
  "cedar:allow-owner",
  "cedar:require-mfa",
  "cedar:deny-unsigned",
  "default-allow",
] as const;

const CHANNELS = ["UI", "CLI", "API", "CI", "A2A", "SOAR"] as const satisfies readonly Channel[];
const DECISIONS = [
  "allow",
  "allow",
  "deny",
  "allow",
  "challenge",
] as const satisfies readonly Decision[];

const HEX = "0123456789abcdef";
const BASE_MS = Date.UTC(2026, 9, 27, 14, 7, 31, 482);

function hex(seed: number, length: number): string {
  let value = Math.imul(seed, 2_654_435_761) >>> 0;
  let out = "";
  for (let index = 0; index < length; index += 1) {
    value = (Math.imul(value, 1_664_525) + 1_013_904_223) >>> 0;
    out += HEX[(value >>> 24) % HEX.length];
  }
  return out;
}

function stamp(ms: number): string {
  const iso = new Date(ms).toISOString();
  return `${iso.slice(0, 10).split("-").reverse().join(".")} ${iso.slice(11, 23)}`;
}

export const AUDIT_EVENTS: readonly AuditEvent[] = Array.from({ length: 50 }, (_, index) => {
  const actor = ACTORS[index % ACTORS.length] ?? ACTORS[0];
  const action = ACTIONS[(index * 3) % ACTIONS.length] ?? ACTIONS[0];
  return {
    id: `evt-${index + 1}`,
    time: stamp(BASE_MS - index * 41_731 - (index % 7) * 113),
    actor,
    action,
    resource: RESOURCES[(index * 5) % RESOURCES.length] ?? RESOURCES[0],
    decision: DECISIONS[index % DECISIONS.length] ?? "allow",
    rule: RULES[(index * 2) % RULES.length] ?? RULES[0],
    channel: CHANNELS[(index * 5) % CHANNELS.length] ?? "UI",
    requestId: `${hex(index + 1, 8)}-${hex(index + 11, 4)}-7${hex(index + 21, 3)}-${hex(index + 31, 4)}-${hex(index + 41, 12)}`,
  };
});

export const PREV_HASH = "e41c07a95b3d28f60c1a9d47e8b5f03a62d91c7e4b08a5f3d26c19e7b40a8d52";
export const EVENT_HASH = "7b02f9d6a1e4c85307bd92f1ae60c4d8153e7a9b02c6f48d1e57a3b09c2d6f81";
