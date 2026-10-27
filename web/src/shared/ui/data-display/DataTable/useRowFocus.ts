import { useVirtualizer } from "@tanstack/react-virtual";
import { type RefObject, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

const INTERACTIVE = "a,button,input,select,textarea,[role=button],[role=link],[role=checkbox]";
const ROW_HEIGHT = 44;
const VIRTUALIZE_AFTER = 200;
const OVERSCAN = 10;
const INITIAL_RECT = { width: 1024, height: 640 };

type Item = { index: number };

/** Окно строк: длинный список виртуализируется, короткий рисуется целиком. */
export function useRowWindow(count: number, scrollRef: RefObject<HTMLElement | null>) {
  const virtual = count > VIRTUALIZE_AFTER;
  const virtualizer = useVirtualizer({
    count: virtual ? count : 0,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: OVERSCAN,
    initialRect: INITIAL_RECT,
  });
  const scrollToIndex = useCallback(
    (index: number) => virtualizer.scrollToIndex(index, { behavior: "auto" }),
    [virtualizer],
  );
  if (!virtual) {
    const items: Item[] = Array.from({ length: count }, (_, index) => ({ index }));
    return { virtual, items, padTop: 0, padBottom: 0, scrollToIndex };
  }
  const items = virtualizer.getVirtualItems();
  return {
    virtual,
    items,
    padTop: items[0]?.start ?? 0,
    padBottom: virtualizer.getTotalSize() - (items.at(-1)?.end ?? 0),
    scrollToIndex,
  };
}

type Options = {
  scrollRef: RefObject<HTMLElement | null>;
  ids: readonly string[];
  /** Прокрутка к строке вне окна виртуализации; без виртуализации не нужна. */
  scrollToIndex: ((index: number) => void) | undefined;
};

function focusElement(element: HTMLElement, inner: boolean): void {
  (inner ? element.querySelector<HTMLElement>(INTERACTIVE) : element)?.focus();
}

/**
 * Фокус на строках таблицы. Строки вне окна виртуализации не в DOM, поэтому фокус откладывается
 * до рендера, в котором нужная строка появилась.
 */
export function useRowFocus({ scrollRef, ids, scrollToIndex }: Options) {
  const [activeId, setActiveId] = useState<string | null>(null);
  const pending = useRef<{ id: string; inner: boolean } | null>(null);

  const find = useCallback(
    (id: string) =>
      Array.from(scrollRef.current?.querySelectorAll<HTMLElement>("tr[data-row-id]") ?? []).find(
        (element) => element.getAttribute("data-row-id") === id,
      ),
    [scrollRef],
  );

  /** `inner`: сама строка не фокусируема, фокус уходит на первый интерактивный элемент внутри. */
  const focusAt = useCallback(
    (index: number, inner = false) => {
      const id = ids[Math.min(Math.max(index, 0), ids.length - 1)];
      if (id === undefined) {
        return;
      }
      pending.current = { id, inner };
      scrollToIndex?.(ids.indexOf(id));
      const element = find(id);
      if (element) {
        pending.current = null;
        focusElement(element, inner);
      }
    },
    [ids, find, scrollToIndex],
  );

  useLayoutEffect(() => {
    const wanted = pending.current;
    const element = wanted ? find(wanted.id) : undefined;
    if (wanted && element) {
      pending.current = null;
      focusElement(element, wanted.inner);
    }
  });

  return { activeId, setActiveId, focusAt };
}

type LoadMoreOptions = {
  rowCount: number;
  hasLoadMore: boolean;
  focusAt: (index: number, inner?: boolean) => void;
  /** Строки не фокусируемы сами: фокус уходит на их первый интерактивный элемент. */
  inner: boolean;
};

/**
 * После догрузки кнопка может исчезнуть вместе с фокусом: возвращаем фокус на первую новую строку
 * (если новых нет, на последнюю). Возвращает обработчик, который запоминает число строк до нажатия.
 */
export function useLoadMoreFocus({ rowCount, hasLoadMore, focusAt, inner }: LoadMoreOptions) {
  const from = useRef<number | null>(null);
  const markLoad = useCallback(() => {
    from.current = rowCount;
  }, [rowCount]);
  useEffect(() => {
    const before = from.current;
    if (before === null || (hasLoadMore && rowCount === before)) {
      return;
    }
    from.current = null;
    if (!hasLoadMore) {
      focusAt(rowCount > before ? before : before - 1, inner);
    }
  }, [rowCount, hasLoadMore, focusAt, inner]);
  return markLoad;
}
