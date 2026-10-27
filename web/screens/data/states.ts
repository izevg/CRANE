import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { i18n } from "~/shared/lib/i18n";
import type { Tone } from "~/shared/ui/primitives";
import type { ActorKind, Decision } from "./audit";
import type { VersionState } from "./versions";

type Entry = { tone: Tone; label: MessageDescriptor };

/** Словарь интерфейса: тон и подпись состояния версии в одном месте, а не в данных и не в каждом экране. */
const VERSION_STATE = {
  released: { tone: "success", label: msg`Выпущена` },
  quarantined: { tone: "warning", label: msg`Карантин` },
  blocked: { tone: "error", label: msg`Заблокирована` },
  deleted: { tone: "neutral", label: msg`Удалена` },
} as const satisfies Record<VersionState, Entry>;

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

/** Пропсы `StatusBadge`: подпись берётся на активной локали в момент рендера. */
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
