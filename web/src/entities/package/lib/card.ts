import { PRIORITY, type SearchItem } from "~/shared/api";
import type { Tone } from "~/shared/ui/primitives";

type SearchPriority = Extract<keyof typeof PRIORITY, string>;

export type RiskPill = { priority: SearchPriority; count: number; tone: Tone };

/**
 * Пилюля риска результата поиска. Только при `risk != null` (поле и необязательное, и nullable): пилюли «Чисто»
 * нет (patterns §6 MUST), отсутствие находок не рисуется вовсе.
 */
export function riskPill(item: Pick<SearchItem, "risk">): RiskPill | null {
  const { risk } = item;
  if (risk === undefined || risk === null) {
    return null;
  }
  return {
    priority: risk.max_open_priority,
    count: risk.open_count,
    tone: PRIORITY[risk.max_open_priority].tone,
  };
}

type Typosquat = { similarTo: string; distance: number };

/** Кандидат typosquatting, если сервер его отдал; `null` и отсутствие поля — «нет». */
export function typosquatOf(item: Pick<SearchItem, "typosquat">): Typosquat | null {
  const { typosquat } = item;
  if (typosquat === undefined || typosquat === null) {
    return null;
  }
  return { similarTo: typosquat.similar_to, distance: typosquat.distance };
}

/** Устаревший пакет рисует бейдж; активный и EOL-состояние отдельного бейджа в карточке не имеют. */
export function showsLifecycleBadge(item: Pick<SearchItem, "lifecycle">): boolean {
  return item.lifecycle.state === "deprecated";
}
