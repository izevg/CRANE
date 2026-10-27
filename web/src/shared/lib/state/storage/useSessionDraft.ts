import { useCallback, useState } from "react";
import { readStored, removeStored, writeStored } from "./storage";

/** Все черновики живут под общим префиксом и не пересекаются с чужими записями `sessionStorage`. */
const PREFIX = "crane.draft.";

const isString = (value: string): value is string => typeof value === "string";

/** Хранилище держит строки: структуру черновика сериализует хук, повреждённый JSON — «черновика нет». */
function parseDraft<T>(
  raw: string | undefined,
  isValid: (value: unknown) => value is T,
): T | undefined {
  if (raw === undefined) {
    return undefined;
  }
  try {
    const parsed: unknown = JSON.parse(raw);
    return isValid(parsed) ? parsed : undefined;
  } catch {
    return undefined;
  }
}

/**
 * Черновик формы в `sessionStorage` на время редиректа (например, step-up в Keycloak): введённое не теряется
 * (WCAG 2.2 3.3.7). Ключ — идентификатор формы без префикса: запись хранится под `crane.draft.<key>`. Возвращает
 * `[черновик, сохранить, очистить]`. Очистку после успешного повтора действия вызывает сам вызывающий: хук не знает, когда повтор удался.
 */
export function useSessionDraft<T>(
  formId: string,
  isValid: (value: unknown) => value is T,
): readonly [T | undefined, (value: T) => void, () => void] {
  const key = `${PREFIX}${formId}`;
  const [draft, setDraft] = useState<T | undefined>(() =>
    parseDraft(readStored("session", key, isString), isValid),
  );
  const save = useCallback(
    (value: T) => {
      writeStored("session", key, JSON.stringify(value));
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
