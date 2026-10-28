import { useSearch } from "@tanstack/react-router";
import { ProblemState } from "~/shared/ui/status";

/**
 * A1 (архетип K): без шелла, карточка по центру. До v0.4.0 входа (BFF) нет, поэтому действия нет: кнопка не
 * рисуется, а подсказка в `ProblemState` объясняет, что вход появится в следующей версии.
 */
export function LoginPage() {
  // Поиск страницы валидирует маршрут (`parseLoginSearch`); `strict: false` не привязывает слайс к id родителя в app.
  const { reason } = useSearch({ strict: false });
  return (
    <div className="rounded-xl border border-border bg-card shadow-card">
      <ProblemState
        headingLevel={1}
        problem={reason === undefined ? { kind: "session" } : { kind: "session", reason }}
      />
    </div>
  );
}
