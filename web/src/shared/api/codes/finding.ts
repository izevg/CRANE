import type { Tone } from "~/shared/ui/primitives";
import type { Priority } from "../generated/admin.gen";

type PriorityEntry = { tone: Tone; strong?: true };

/**
 * Приоритет находки (patterns §10): P0 и P1 — error, P2 — warning, P3 — info, P4 — neutral. P0 отличается от P1
 * заполненной точкой (`strong`), потому что тон у них один. Подпись — сам код приоритета.
 */
export const PRIORITY = {
  P0: { tone: "error", strong: true },
  P1: { tone: "error" },
  P2: { tone: "warning" },
  P3: { tone: "info" },
  P4: { tone: "neutral" },
} as const satisfies Record<Priority, PriorityEntry>;
