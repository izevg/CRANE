import { useMemo } from "react";
import { cx } from "~/shared/lib/core";
import { type CharKind, splitSeries } from "./charClasses";

type MachineValueProps = {
  /** Видимый текст (может быть усечён, например «a88d…6973»). */
  value: string;
  /** Полное значение для скринридера, если видимое усечено; по умолчанию равно `value`. */
  fullValue?: string;
  /** Без разметки: на статусных фонах (callout, diff, badge) цвета токенов не рассчитаны. */
  plain?: boolean;
};

// Буквы остаются цветом текста родителя (--foreground, на инверсной поверхности — --inverse-foreground).
const KIND_CLASS = {
  digit: "text-char-digit",
  symbol: "text-char-symbol",
  letter: "",
} satisfies Record<CharKind, string>;

/**
 * Машинное значение (хэш, UUID, префикс токена) с цветом по классам символов, как в менеджерах паролей.
 * Разметка только для глаза: размеченный слой `aria-hidden`, а полное значение лежит одним куском в `sr-only`
 * (и не выделяется мышью, иначе копирование захватило бы его дважды). Серии одного класса — один span без
 * пробелов между ними, поэтому выделение и копирование дают точную строку, а раскладка не сдвигается.
 * Режим forced-colors не отключаем (`forced-color-adjust` не трогаем): система сама красит текст одним цветом.
 */
export function MachineValue({ value, fullValue, plain = false }: MachineValueProps) {
  const series = useMemo(() => (plain ? null : splitSeries(value)), [plain, value]);
  if (series === null && fullValue === undefined) {
    return value;
  }
  return (
    <>
      <span
        aria-hidden="true"
        data-slot="machine-value"
        className="selection:bg-[Highlight] selection:text-[HighlightText]"
      >
        {series === null
          ? value
          : series.map((item) => (
              <span key={item.start} className={cx(KIND_CLASS[item.kind])}>
                {item.text}
              </span>
            ))}
      </span>
      <span className="sr-only select-none">{fullValue ?? value}</span>
    </>
  );
}
