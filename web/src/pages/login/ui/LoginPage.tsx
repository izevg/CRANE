import { SIGN_IN_AVAILABLE_IN } from "~/shared/api";
import { ProblemState } from "~/shared/ui/status";

/**
 * A1 (архетип K): без шелла, карточка по центру. До v0.4.0 входа (BFF) нет, поэтому действия нет: кнопка не
 * рисуется, а подсказка называет версию, в которой вход появится. Причину завершения сессии (`reason`) страница
 * не показывает: без входа она ничего не даёт; параметр разобран маршрутом и понадобится вместе со входом.
 */
export function LoginPage() {
  return (
    <div className="rounded-xl border border-border bg-card shadow-card">
      <ProblemState
        headingLevel={1}
        problem={{ kind: "session", availableIn: SIGN_IN_AVAILABLE_IN }}
      />
    </div>
  );
}
