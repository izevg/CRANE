import { type RefObject, useEffect } from "react";

/**
 * Следит за горизонтальным переполнением контейнера прокрутки и пишет его в data-атрибуты `wrapper`
 * (`data-overflow-start|end`): затухания по краям включаются классами `group-data-[overflow-end=true]/…`.
 * React не перерисовывается: атрибуты меняются только при смене булевых значений.
 */
export function useScrollOverflow(
  scrollRef: RefObject<HTMLElement | null>,
  wrapperRef: RefObject<HTMLElement | null>,
): void {
  useEffect(() => {
    const scroller = scrollRef.current;
    const wrapper = wrapperRef.current;
    if (!(scroller && wrapper)) {
      return;
    }
    const last: { start: boolean | null; end: boolean | null } = { start: null, end: null };
    const update = () => {
      const start = scroller.scrollLeft > 0;
      const end = scroller.scrollLeft + scroller.clientWidth < scroller.scrollWidth - 1;
      if (start !== last.start) {
        last.start = start;
        wrapper.setAttribute("data-overflow-start", String(start));
      }
      if (end !== last.end) {
        last.end = end;
        wrapper.setAttribute("data-overflow-end", String(end));
      }
    };
    update();
    scroller.addEventListener("scroll", update, { passive: true });
    const observer = new ResizeObserver(update);
    observer.observe(scroller);
    const content = scroller.firstElementChild;
    if (content) {
      observer.observe(content);
    }
    return () => {
      scroller.removeEventListener("scroll", update);
      observer.disconnect();
    };
  }, [scrollRef, wrapperRef]);
}
