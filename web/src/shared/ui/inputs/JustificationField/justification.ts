/** Из спеки `Justification` (common.yaml): строка, `minLength` 20, `maxLength` 2000, trimmed, в code points. */
export const JUSTIFICATION_MIN = 20;
export const JUSTIFICATION_MAX = 2000;

/**
 * Длина в code points после обрезки пробелов по краям. `String.length` считает UTF-16 единицы, и эмодзи
 * весил бы вдвое; спека меряет символами, поэтому `Array.from`.
 */
export function justificationLength(value: string): number {
  return Array.from(value.trim()).length;
}

export type JustificationStatus = "short" | "ok" | "long";

export function justificationStatus(value: string): JustificationStatus {
  const length = justificationLength(value);
  if (length < JUSTIFICATION_MIN) {
    return "short";
  }
  return length > JUSTIFICATION_MAX ? "long" : "ok";
}

/** Единственный предикат «обоснование годится»: им пользуются и поле, и кнопка действия (экраны сами не считают). */
export function isJustified(value: string): boolean {
  return justificationStatus(value) === "ok";
}

/** Сколько символов осталось до предела; не меньше нуля. */
export function justificationLeft(value: string): number {
  return Math.max(JUSTIFICATION_MAX - justificationLength(value), 0);
}
