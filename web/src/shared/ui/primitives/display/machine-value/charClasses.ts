/** Класс символа машинного значения: цифры и знаки красятся, буквы и всё остальное остаются цветом текста. */
export type CharKind = "digit" | "symbol" | "letter";

/** `start` — позиция первого символа серии: стабильный ключ списка. */
export type Series = { kind: CharKind; text: string; start: number };

/** Длиннее — без разметки: сотни цветных span стоят дороже, чем помогают глазу. */
export const MAX_MARKED_LENGTH = 64;

const ELLIPSIS = "…";
const ASCII_PRINTABLE_FIRST = 0x21;
const ASCII_PRINTABLE_LAST = 0x7e;
const ZERO = 0x30;
const NINE = 0x39;
const UPPER_A = 0x41;
const UPPER_Z = 0x5a;
const LOWER_A = 0x61;
const LOWER_Z = 0x7a;

/**
 * digit — [0-9]; symbol — остальные печатные ASCII без пробела и «…» (знак усечения); letter — буквы A-Za-z
 * и всё прочее (пробел, перевод строки, кириллица): они остаются цветом текста.
 */
export function classOf(char: string): CharKind {
  const code = char.codePointAt(0) ?? 0;
  if (code >= ZERO && code <= NINE) {
    return "digit";
  }
  const letter = (code >= UPPER_A && code <= UPPER_Z) || (code >= LOWER_A && code <= LOWER_Z);
  if (letter) {
    return "letter";
  }
  if (char === ELLIPSIS || (code >= ASCII_PRINTABLE_FIRST && code <= ASCII_PRINTABLE_LAST)) {
    return "symbol";
  }
  return "letter";
}

/** Серии подряд идущих символов одного класса: «a1_B-9» → a | 1 | _ | B | - | 9. `null` — слишком длинное значение. */
export function splitSeries(value: string): Series[] | null {
  const chars = Array.from(value);
  if (chars.length > MAX_MARKED_LENGTH) {
    return null;
  }
  const series: Series[] = [];
  chars.forEach((char, start) => {
    const kind = classOf(char);
    const last = series.at(-1);
    if (last?.kind === kind) {
      last.text += char;
    } else {
      series.push({ kind, text: char, start });
    }
  });
  return series;
}
