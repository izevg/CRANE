// i18n: не мигрирован
import { PauseIcon, PlayIcon } from "lucide-react";
import { useCallback } from "react";
import { cx, type WithClassName } from "~/shared/lib/core";
import { Button } from "../../primitives";

const LABEL = "Пауза автообновления";

type AutoRefreshToggleProps = WithClassName<{
  /** `true` — автообновление на паузе. */
  paused: boolean;
  onPausedChange: (paused: boolean) => void;
}>;

/**
 * Переключатель паузы автообновления (WCAG 2.2.2): кнопка с `aria-pressed`, имя не меняется.
 * Сам опрос живёт у вызывающего; компонент только сообщает о выборе.
 */
export function AutoRefreshToggle({ paused, onPausedChange, className }: AutoRefreshToggleProps) {
  const toggle = useCallback(() => onPausedChange(!paused), [onPausedChange, paused]);
  const Glyph = paused ? PlayIcon : PauseIcon;
  return (
    <Button
      variant="secondary"
      size="sm"
      aria-pressed={paused}
      className={cx(className)}
      onClick={toggle}
    >
      <Glyph aria-hidden="true" />
      {LABEL}
    </Button>
  );
}
