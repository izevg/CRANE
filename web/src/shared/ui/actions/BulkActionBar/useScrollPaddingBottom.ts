import { type RefObject, useEffect, useId } from "react";

/** Владельцы резерва: id панели → нужная ей высота. На `html` действует максимум, чтобы панели не перетирали друг друга. */
const owners = new Map<string, number>();

function publish(): void {
  const root = document.documentElement;
  if (owners.size === 0) {
    root.style.removeProperty("scroll-padding-bottom");
    return;
  }
  root.style.scrollPaddingBottom = `${Math.max(...owners.values())}px`;
}

/**
 * Пока `active`, резервирует снизу область прокрутки документа под плавающую панель (WCAG 2.4.11):
 * `scroll-padding-bottom` = высота панели + `gap`, чтобы сфокусированный элемент не прятался под ней.
 * Прокручивается документ (`html`), а не контейнер шелла. Свойство снимается, когда владельцев не осталось.
 */
export function useScrollPaddingBottom(
  ref: RefObject<HTMLElement | null>,
  active: boolean,
  gap: number,
): void {
  const id = useId();
  useEffect(() => {
    const element = ref.current;
    if (!(active && element)) {
      return;
    }
    const apply = () => {
      owners.set(id, Math.ceil(element.getBoundingClientRect().height) + gap);
      publish();
    };
    apply();
    const observer = new ResizeObserver(apply);
    observer.observe(element);
    return () => {
      observer.disconnect();
      owners.delete(id);
      publish();
    };
  }, [ref, active, gap, id]);
}
