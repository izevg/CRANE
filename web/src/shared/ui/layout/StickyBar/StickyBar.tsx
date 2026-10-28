import { type ReactNode, useLayoutEffect, useRef } from "react";
import { useScrollPaddingBottom } from "~/shared/lib/a11y";
import { cx, type WithClassName } from "~/shared/lib/core";
import { useExitPresence } from "./useExitPresence";

type StickyBarProps<T> = WithClassName<{
  /** Содержимое, пока панель нужна; `null` — панель уходит. На выходе рисуется последнее непустое значение. */
  value: T | null;
  children: (value: T) => ReactNode;
  /**
   * Куда перевести фокус, если на выходе он остался внутри панели (WCAG 2.4.3): иначе `inert` выбросил бы его на
   * `<body>`. Обычно это заголовок карточки, форма или первое поле. `null` оставляет фокус браузеру.
   */
  returnFocus: () => HTMLElement | null;
}>;

/** Совпадает с `--dur-fast`: за это время отыгрывает выход, потом панель размонтируется. */
const EXIT_MS = 150;

/** Запас над панелью, чтобы сфокусированный элемент не упирался в её край. */
const SCROLL_PADDING_GAP = 16;

/**
 * Липкая панель у нижнего края области содержимого (панель сохранения, подвал мастера).
 * Пока смонтирована, документ резервирует под неё `scroll-padding-bottom` (WCAG 2.4.11). Появляется сдвигом и
 * фейдом, уходит быстрее; при reduced motion сдвига нет. На выходе становится `inert` и отдаёт фокус `returnFocus`.
 */
export function StickyBar<T>({ value, children, returnFocus, className }: StickyBarProps<T>) {
  const barRef = useRef<HTMLDivElement>(null);
  const presence = useExitPresence(value, EXIT_MS);
  const { mounted, leaving } = presence;
  useScrollPaddingBottom(barRef, mounted, SCROLL_PADDING_GAP);
  // Фокус переводим раньше, чем элемент станет inert, иначе он уже потерян; атрибут ставим руками по той же причине
  useLayoutEffect(() => {
    const bar = barRef.current;
    if (bar === null) {
      return;
    }
    if (leaving && bar.contains(document.activeElement)) {
      returnFocus()?.focus();
    }
    bar.inert = leaving;
  }, [leaving, returnFocus]);

  if (!mounted || presence.value === undefined) {
    return null;
  }
  return (
    <div
      ref={barRef}
      data-leaving={leaving ? "" : undefined}
      className={cx(
        "sticky bottom-0 z-10 border-border border-t bg-card px-4 py-3 text-body shadow-card transition-[opacity,translate] duration-(--dur-base) ease-(--ease-out) starting:translate-y-2 starting:opacity-0 motion-reduce:starting:translate-y-0 data-leaving:translate-y-2 data-leaving:opacity-0 data-leaving:duration-(--dur-fast) motion-reduce:data-leaving:translate-y-0",
        className,
      )}
    >
      {children(presence.value)}
    </div>
  );
}
