import { useCallback } from "react";
import { cx } from "~/shared/lib/core";
import { formatCount, formatNumber } from "~/shared/lib/text";
import { ToggleGroup, ToggleGroupItem } from "~/shared/ui/primitives";

/** Счётчик: число или усечённый подсчёт сервера («1 000+»). */
type SegmentCount = number | { count: number; capped: boolean };

type SegmentedItem<V extends string> = {
  value: V;
  label: string;
  count?: SegmentCount;
};

type SegmentedControlProps<V extends string> = {
  items: readonly SegmentedItem<V>[];
  value: V;
  onValueChange: (value: V) => void;
  /** Доступное имя группы. */
  label: string;
  /** Только размещение (margin/width/grid) внешнего узла. */
  className?: string;
};

const SEPARATOR = " ";

function formatSegmentCount(count: SegmentCount): string {
  return typeof count === "number" ? formatNumber(count) : formatCount(count);
}

/**
 * Одиночный выбор поверх toggle-group. Счётчик входит в доступное имя кнопки («Выпущенные 41»);
 * выбор нельзя снять повторным нажатием: группа всегда показывает один сегмент.
 */
export function SegmentedControl<V extends string>({
  items,
  value,
  onValueChange,
  label,
  className,
}: SegmentedControlProps<V>) {
  const handleChange = useCallback(
    (next: readonly string[]) => {
      const picked = items.find((item) => item.value === next[0]);
      if (picked !== undefined) {
        onValueChange(picked.value);
      }
    },
    [items, onValueChange],
  );
  return (
    <ToggleGroup
      aria-label={label}
      variant="outline"
      size="sm"
      value={[value]}
      onValueChange={handleChange}
      className={cx(className)}
    >
      {items.map((item) => (
        <ToggleGroupItem key={item.value} value={item.value}>
          {item.label}
          {item.count === undefined ? null : (
            <>
              {SEPARATOR}
              <span className="text-caption text-muted-foreground tabular-nums">
                {formatSegmentCount(item.count)}
              </span>
            </>
          )}
        </ToggleGroupItem>
      ))}
    </ToggleGroup>
  );
}
