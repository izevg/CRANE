import { useMemo } from "react";
import { cx } from "~/shared/lib/core";
import { type CharKind, splitSeries } from "./charClasses";

type MachineValueProps = {
  /** Видимый текст (может быть усечён, например «a88d…6973»). */
  value: string;
  /** Что произнесёт скринридер, если видимое значение усечено; по умолчанию равно `value`. */
  spoken?: string;
  /** Размечать цветом по классам символов; `false` на статусных фонах (callout, diff, badge): цвета токенов там не рассчитаны. */
  marked?: boolean;
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
export function MachineValue({ value, spoken, marked = true }: MachineValueProps) {
  const series = useMemo(() => (marked ? splitSeries(value) : null), [marked, value]);
  if (series === null && spoken === undefined) {
    return value;
  }
  return (
    <>
      <span
        aria-hidden="true"
        data-slot="machine-value"
        className="font-mono selection:bg-[Highlight] selection:text-[HighlightText]"
      >
        {series === null
          ? value
          : series.map((item) => (
              <span key={item.start} className={cx(KIND_CLASS[item.kind])}>
                {item.text}
              </span>
            ))}
      </span>
      <span className="sr-only select-none">{spoken ?? value}</span>
    </>
  );
}
