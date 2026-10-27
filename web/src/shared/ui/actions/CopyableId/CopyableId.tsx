import { CopyButton } from "../CopyButton/CopyButton";

const HEAD = 4;
const TAIL = 4;
const DEFAULT_LABEL = "Скопировать ID";

/** `a88d…6973`; значение, которое сокращение не сделало бы короче, остаётся целиком. */
function shorten(value: string): string {
  const chars = Array.from(value);
  if (chars.length <= HEAD + TAIL + 1) {
    return value;
  }
  return `${chars.slice(0, HEAD).join("")}…${chars.slice(-TAIL).join("")}`;
}

type CopyableIdProps = {
  value: string;
  /** Показать значение целиком (с переносом). */
  full?: boolean;
  /** Начало доступного имени кнопки; к нему добавляется полное значение. */
  label?: string;
};

/**
 * Идентификатор с усечением в середине. Полное значение лежит в `title`, в доступном имени кнопки
 * и в скрытом для глаза тексте; копируется всегда полное значение.
 */
export function CopyableId({ value, full = false, label = DEFAULT_LABEL }: CopyableIdProps) {
  const shown = full ? value : shorten(value);
  const shortened = shown !== value;
  return (
    <span className="inline-flex max-w-full min-w-0 items-center gap-0.5 font-mono tabular-nums">
      <span title={value} className="min-w-0 wrap-anywhere">
        <span aria-hidden={shortened || undefined}>{shown}</span>
        {shortened ? <span className="sr-only">{value}</span> : null}
      </span>
      <CopyButton value={value} label={`${label}: ${value}`} className="shrink-0" />
    </span>
  );
}
