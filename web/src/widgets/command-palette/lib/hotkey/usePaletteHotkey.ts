import { useEffect } from "react";

/**
 * Ctrl+K и ⌘K. Проверяем `code`, а не `key`: в русской раскладке на этой клавише «л».
 * Сочетание с модификатором не печатает символ, поэтому срабатывает и внутри полей ввода (как в BRICK):
 * палитра нужна из любого места, а перехватывать здесь нечего. Автоповтор, Alt и Shift игнорируются.
 */
function isPaletteHotkey(event: KeyboardEvent): boolean {
  return (
    event.code === "KeyK" &&
    (event.metaKey || event.ctrlKey) &&
    !event.altKey &&
    !event.shiftKey &&
    !event.repeat
  );
}

/** Глобальный хоткей открытия палитры. Если она уже открыта, вызов идемпотентен: закрытие — это Esc. */
export function usePaletteHotkey(onOpen: () => void): void {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (!isPaletteHotkey(event)) {
        return;
      }
      // Без этого браузер уйдёт в свою строку поиска/адреса.
      event.preventDefault();
      onOpen();
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [onOpen]);
}
