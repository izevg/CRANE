import { useCallback, useState } from "react";
import { readStored, removeStored, writeStored } from "./storage";

/**
 * Черновик формы в `sessionStorage` на время редиректа (например, step-up в Keycloak): введённое не теряется
 * (WCAG 2.2 3.3.7). Возвращает `[черновик, сохранить, очистить]`. Очистку после успешного повтора действия вызывает
 * сам вызывающий: хук не знает, когда повтор удался.
 */
export function useSessionDraft<T>(
  key: string,
  isValid: (value: unknown) => value is T,
): readonly [T | undefined, (value: T) => void, () => void] {
  const [draft, setDraft] = useState<T | undefined>(() => readStored("session", key, isValid));
  const save = useCallback(
    (value: T) => {
      writeStored("session", key, value);
      setDraft(value);
    },
    [key],
  );
  const clear = useCallback(() => {
    removeStored("session", key);
    setDraft(undefined);
  }, [key]);
  return [draft, save, clear] as const;
}
