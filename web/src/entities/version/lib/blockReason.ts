import type { BlockReason, BlockReasonKind, VersionState } from "~/shared/api";

export type ReasonView = {
  kind: BlockReasonKind;
  /** Правило (`rule`) или номер advisory (`emergency`): то, что стоит после «·». */
  detail: string | undefined;
  /** Ближайшая исправленная версия (подсказка), если сервер её знает. */
  fixedIn: string | undefined;
};

/**
 * Причина блокировки, которую нужно показать рядом с состоянием. Только у `blocked`: у остальных состояний
 * поле не используется. `block_reason` в спеке и необязательно, и nullable, оба случая означают «нет».
 */
export function reasonView(
  state: VersionState,
  reason: BlockReason | null | undefined,
): ReasonView | null {
  if (state !== "blocked" || reason == null) {
    return null;
  }
  return {
    kind: reason.kind,
    detail: reason.kind === "emergency" ? reason.advisory_id : reason.rule_id,
    fixedIn: reason.fixed_in,
  };
}
