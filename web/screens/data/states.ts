import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import type { StatusEntry } from "~/shared/api";
import { i18n } from "~/shared/lib/i18n";
import type { Tone } from "~/shared/ui/primitives";
import type { ActorKind, Decision } from "./audit";

/** Словарь интерфейса: тон и подпись решения аудита (состояния версий — в `shared/api`). */
const DECISION = {
  allow: { tone: "success", label: msg`Разрешено` },
  deny: { tone: "error", label: msg`Отказ` },
  challenge: { tone: "warning", label: msg`Нужен MFA` },
} as const satisfies Record<Decision, StatusEntry>;

const ACTOR_KIND = {
  human: msg`Человек`,
  robot: msg`Робот`,
  workload: msg`Workload`,
  agent: msg`Агент`,
} as const satisfies Record<ActorKind, MessageDescriptor>;

type Badge = { tone: Tone; label: string };

export function decisionBadge(decision: Decision): Badge {
  const { tone, label } = DECISION[decision];
  return { tone, label: i18n._(label) };
}

/** Подпись вида актора на активной локали. */
export function actorKindLabel(kind: ActorKind): string {
  return i18n._(ACTOR_KIND[kind]);
}
