import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import type { Tone } from "~/shared/ui/primitives";
import type { Format, Priority, RepositoryKind } from "../generated/admin.gen";

type Entry = { tone: Tone; label: MessageDescriptor };

/**
 * Форматы: нейтральный бейдж, форматы не раскрашиваются разными цветами (цвет без смысла, patterns §9).
 * Названия собственные и не переводятся: сообщение каталога совпадает в обоих языках.
 */
export const FORMAT = {
  npm: { tone: "neutral", label: msg`npm` },
  pypi: { tone: "neutral", label: msg`PyPI` },
  maven: { tone: "neutral", label: msg`Maven` },
  nuget: { tone: "neutral", label: msg`NuGet` },
  cargo: { tone: "neutral", label: msg`Cargo` },
} as const satisfies Record<Format, Entry>;

/** Вид репозитория: «hosted» и «proxy» — термины продукта, нейтральный бейдж. */
export const REPOSITORY_KIND = {
  hosted: { tone: "neutral", label: msg`hosted` },
  proxy: { tone: "neutral", label: msg`proxy` },
} as const satisfies Record<RepositoryKind, Entry>;

/**
 * Приоритет находки (patterns §10): P0 и P1 — error, P2 — warning, P3 — info, P4 — neutral.
 * Слово рядом с кодом («P1 Критично») собирает потребитель.
 */
export const PRIORITY = {
  P0: { tone: "error", label: msg`Чрезвычайный` },
  P1: { tone: "error", label: msg`Критично` },
  P2: { tone: "warning", label: msg`Высокий` },
  P3: { tone: "info", label: msg`Средний` },
  P4: { tone: "neutral", label: msg`Низкий` },
} as const satisfies Record<Priority, Entry>;
