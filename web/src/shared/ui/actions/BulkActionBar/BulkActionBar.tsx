import { XIcon } from "lucide-react";
import { useEffect, useRef } from "react";
import { announce } from "~/shared/lib/a11y";
import { cx, type WithClassName } from "~/shared/lib/core";
import { Button } from "../../primitives";
import { ActionBar } from "../ActionBar/ActionBar";
import type { Action } from "../ActionButton/action";
import { useScrollPaddingBottom } from "./useScrollPaddingBottom";

type BulkActionBarProps = WithClassName<{
  /** Сколько строк выбрано; 0 — панель не рендерится. */
  count: number;
  /** Действия над выбранными: нет `Action` — нет кнопки. */
  actions: readonly Action[];
  onClear: () => void;
}>;

const REGION_LABEL = "Массовые действия";
const CLEAR_LABEL = "Снять выделение";
const SELECTED_PREFIX = "Выбрано ";

/** Панель висит на `bottom-4` (16 px от края); запас над ней ещё 16 px: итого высота + 32. */
const SCROLL_PADDING_GAP = 32;

/**
 * Плавающая панель массовых действий (поверх таблицы, снизу по центру; ниже `md` — на всю ширину).
 * Это `group`, а не `toolbar`: кнопки действий (`ActionButton`) не регистрируются в roving tabindex Base UI,
 * поэтому «Выбрано N», действия и «Снять выделение» идут в обычном Tab-порядке.
 * Пока панель смонтирована, документ резервирует `scroll-padding-bottom` под неё, чтобы она не
 * перекрывала последнюю строку и сфокусированный элемент (WCAG 2.4.11). Изменение числа выбранных
 * объявляется скринридеру через общий live-регион.
 */
export function BulkActionBar({ count, actions, onClear, className }: BulkActionBarProps) {
  const barRef = useRef<HTMLDivElement>(null);
  const visible = count > 0;
  useScrollPaddingBottom(barRef, visible, SCROLL_PADDING_GAP);
  useEffect(() => {
    if (count > 0) {
      announce(`${SELECTED_PREFIX}${count}`);
    }
  }, [count]);

  if (!visible) {
    return null;
  }
  return (
    // biome-ignore lint/a11y/useSemanticElements: <fieldset> — для форм; здесь плавающая панель, нужна именованная группа
    <div
      ref={barRef}
      role="group"
      aria-label={REGION_LABEL}
      className={cx(
        "sticky bottom-4 z-10 mx-auto flex w-full max-w-full flex-wrap items-center gap-3 bg-card px-4 py-2 text-body shadow-card transition-[opacity,translate] duration-(--dur-base) ease-(--ease-out) starting:translate-y-2 starting:opacity-0 motion-reduce:transition-none md:w-fit md:rounded-lg",
        className,
      )}
    >
      <span className="font-semibold tabular-nums">
        {SELECTED_PREFIX}
        {count}
      </span>
      <ActionBar actions={actions} />
      <Button variant="ghost" size="icon-sm" aria-label={CLEAR_LABEL} onClick={onClear}>
        <XIcon aria-hidden="true" />
      </Button>
    </div>
  );
}
