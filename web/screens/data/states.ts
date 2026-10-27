import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { VERSION_STATE, type VersionState } from "~/shared/api";
import { i18n } from "~/shared/lib/i18n";
import type { Tone } from "~/shared/ui/primitives";
import type { ActorKind, Decision } from "./audit";

type Entry = { tone: Tone; label: MessageDescriptor };

/** Словарь интерфейса: тон и подпись решения аудита (состояния версий — в `shared/api`). */
const DECISION = {
  allow: { tone: "success", label: msg`Разрешено` },
  deny: { tone: "error", label: msg`Отказ` },
  challenge: { tone: "warning", label: msg`Нужен MFA` },
} as const satisfies Record<Decision, Entry>;

const ACTOR_KIND = {
  human: msg`Человек`,
  robot: msg`Робот`,
  workload: msg`Workload`,
  agent: msg`Агент`,
} as const satisfies Record<ActorKind, MessageDescriptor>;

type Badge = { tone: Tone; label: string };

/** Пропсы `StatusBadge` для состояния версии из словаря API; подпись берётся на активной локали в момент рендера. */
export function versionBadge(state: VersionState): Badge {
  const { tone, label } = VERSION_STATE[state];
  return { tone, label: i18n._(label) };
}

export function decisionBadge(decision: Decision): Badge {
  const { tone, label } = DECISION[decision];
  return { tone, label: i18n._(label) };
}

/** Подпись вида актора на активной локали. */
export function actorKindLabel(kind: ActorKind): string {
  return i18n._(ACTOR_KIND[kind]);
}
