import { msg } from "@lingui/core/macro";
import type { ComponentState, PlatformMode } from "~/shared/api";
import { i18n } from "~/shared/lib/i18n";
import type { Tone } from "~/shared/ui/primitives";

type Entry = { tone: Tone; label: ReturnType<typeof msg> };

/** Режим платформы в шапке: подпись всегда текстом, тон не единственный носитель смысла. */
export const MODE_STATUS = {
  normal: { tone: "success", label: msg`Режим: норма` },
  read_only: { tone: "warning", label: msg`Режим: только чтение` },
  degraded: { tone: "warning", label: msg`Режим: деградация` },
} satisfies Record<PlatformMode, Entry>;

export const COMPONENT_STATUS = {
  ok: { tone: "success", label: msg`Работает` },
  degraded: { tone: "warning", label: msg`Деградация` },
  down: { tone: "error", label: msg`Недоступен` },
} satisfies Record<ComponentState, Entry>;

const UPSTREAM_PREFIX = "upstream:";

/**
 * Деградация из `degraded[]`: `semantic`, `ai`, `upstream:<name>`. Неизвестный код показывается как есть,
 * чтобы новое значение сервера не терялось молча.
 */
export function degradedLabel(code: string): string {
  if (code === "semantic") {
    return i18n._(msg`Семантический поиск недоступен`);
  }
  if (code === "ai") {
    return i18n._(msg`Ассистенты недоступны`);
  }
  if (code.startsWith(UPSTREAM_PREFIX)) {
    const name = code.slice(UPSTREAM_PREFIX.length);
    return i18n._(msg`Upstream ${name} недоступен`);
  }
  return code;
}
