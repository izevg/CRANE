const INITIALS_COUNT = 2;
const WHITESPACE = /\s+/;

/** Инициалы для аватара: первые буквы первых двух слов имени («Иванов И.» → «ИИ»). */
export function initialsOf(name: string): string {
  return name
    .split(WHITESPACE)
    .filter((word) => word !== "")
    .slice(0, INITIALS_COUNT)
    .map((word) => Array.from(word)[0] ?? "")
    .join("")
    .toLocaleUpperCase();
}

export type ColorMode = "light" | "dark" | "system";

/** Порядок пунктов переключателя темы. */
export const COLOR_MODES: readonly ColorMode[] = ["light", "dark", "system"];

export function isColorMode(value: unknown): value is ColorMode {
  return COLOR_MODES.some((mode) => mode === value);
}
