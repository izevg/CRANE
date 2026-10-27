import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import type { Tone } from "~/shared/ui/primitives";
import type { BlockReasonKind, LifecycleState, VersionState } from "../generated/admin.gen";

type Entry = { tone: Tone; label: MessageDescriptor };

/**
 * Словари интерфейса для кодов API: подпись и тон по значению enum. Полноту даёт тип `Record`: новое значение в
 * спеке не соберётся, пока для него нет подписи. Машинный код наружу не показывается (patterns §9).
 * Перевод берётся при отрисовке (`_(label)`), поэтому таблица безопасна на уровне модуля.
 */
export const VERSION_STATE = {
  quarantined: { tone: "warning", label: msg`Карантин` },
  released: { tone: "success", label: msg`Выпущена` },
  blocked: { tone: "error", label: msg`Заблокирована` },
  deleted: { tone: "neutral", label: msg`Удалена` },
} as const satisfies Record<VersionState, Entry>;

export const LIFECYCLE_STATE = {
  active: { tone: "success", label: msg`Активен` },
  deprecated: { tone: "warning", label: msg`Устарел` },
  eol: { tone: "neutral", label: msg`EOL` },
} as const satisfies Record<LifecycleState, Entry>;

/** Начало причины блокировки; после него через «·» идёт правило или номер advisory. */
export const BLOCK_REASON_KIND = {
  rule: msg`По правилу`,
  emergency: msg`Экстренно`,
} as const satisfies Record<BlockReasonKind, MessageDescriptor>;
