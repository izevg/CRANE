import { useCallback } from "react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "~/shared/ui/primitives";

type FilterOption = { value: string; label: string };

type FilterSelectProps = {
  /** Доступное имя триггера. */
  label: string;
  /** Подпись пункта «все», например «Все форматы»; ему соответствует значение `null`. */
  allLabel: string;
  value: string | null;
  options: readonly FilterOption[];
  onValueChange: (value: string | null) => void;
  size?: "sm" | "default";
  /** Только размещение (margin/width/grid); без него ширина фиксирована. */
  className?: string;
};

const ALL_KEY = "all";
const DEFAULT_WIDTH = "w-52 max-w-full";

type Item = { value: string | null; label: string };

/**
 * Фильтр «все / конкретное значение». Значение, которого нет в списке (список ещё не загрузился
 * или ключ удалён), показывается как есть — ключом, чтобы фильтр не выглядел сброшенным.
 */
export function FilterSelect({
  label,
  allLabel,
  value,
  options,
  onValueChange,
  size = "default",
  className,
}: FilterSelectProps) {
  const known = value === null || options.some((option) => option.value === value);
  const items: readonly Item[] = [
    { value: null, label: allLabel },
    ...(known ? [] : [{ value, label: value }]),
    ...options,
  ];
  const handleChange = useCallback((next: string | null) => onValueChange(next), [onValueChange]);
  return (
    <Select items={items} value={value} onValueChange={handleChange}>
      <SelectTrigger size={size} aria-label={label} className={className ?? DEFAULT_WIDTH}>
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        {items.map((item) => (
          <SelectItem key={item.value ?? ALL_KEY} value={item.value}>
            {item.label}
          </SelectItem>
        ))}
      </SelectContent>
    </Select>
  );
}
