import type { BlockReason, BlockReasonKind, VersionState } from "~/shared/api";

export type ReasonView = {
  kind: BlockReasonKind;
  /** Правило (`rule`) или номер advisory (`emergency`): то, что стоит после «·». */
  detail: string | undefined;
  /** Ближайшая исправленная версия, если сервер её знает. */
  fixedIn: string | undefined;
  /** Идентификатор ресурса экстренной блокировки (`EmergencyBlock`); у блокировки по правилу его нет. */
  blockId: string | undefined;
};

/**
 * Причина блокировки, которую нужно показать рядом с состоянием. Только у `blocked`: у остальных состояний
 * поле не используется. `block_reason` в спеке обязательное, но nullable (`BlockReason | null`), `null` значит
 * «причины нет». `BlockReason` — объединение по `kind`: у `rule` есть `rule_id`, у `emergency` — `block_id` и
 * `advisory_id`.
 */
export function reasonView(state: VersionState, reason: BlockReason | null): ReasonView | null {
  if (state !== "blocked" || reason === null) {
    return null;
  }
  if (reason.kind === "emergency") {
    return {
      kind: reason.kind,
      detail: reason.advisory_id,
      fixedIn: reason.fixed_in,
      blockId: reason.block_id,
    };
  }
  return {
    kind: reason.kind,
    detail: reason.rule_id,
    fixedIn: reason.fixed_in,
    blockId: undefined,
  };
}
