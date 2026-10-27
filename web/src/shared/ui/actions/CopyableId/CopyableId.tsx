import { useLingui } from "@lingui/react";
import { MachineValue } from "../../primitives";
import { CopyButton } from "../CopyButton/CopyButton";
import { DEFAULT_LABEL } from "./messages";

const HEAD = 4;
const TAIL = 4;

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
  /** Без цветовой разметки классов символов: для значений не «машинного» вида (PURL, версия) и статусных фонов. */
  plain?: boolean;
};

/**
 * Идентификатор с усечением в середине. Полное значение лежит в `title`, в доступном имени кнопки
 * и в скрытом для глаза тексте; копируется всегда полное значение.
 */
export function CopyableId({ value, full = false, label, plain = false }: CopyableIdProps) {
  const { _ } = useLingui();
  const copyLabel = label ?? _(DEFAULT_LABEL);
  const shown = full ? value : shorten(value);
  const shortened = shown !== value;
  return (
    <span className="inline-flex max-w-full min-w-0 items-center gap-0.5 font-mono tabular-nums">
      <span title={value} className="min-w-0 wrap-anywhere">
        <MachineValue value={shown} plain={plain} {...(shortened ? { fullValue: value } : {})} />
      </span>
      <CopyButton value={value} label={`${copyLabel}: ${value}`} className="shrink-0" />
    </span>
  );
}
