import { useSessionDraft } from "~/shared/lib/state";

const PREFIX = "crane.stepup.";

/**
 * Черновик формы на время редиректа в Keycloak. Тонкая обёртка над `useSessionDraft` нужна ради одного: ключи
 * step-up живут под общим префиксом `crane.stepup.` и не пересекаются с чужими записями sessionStorage.
 * После успешного повтора действия вызывающий вызывает `clear`.
 */
export function useStepUpDraft<T>(
  formId: string,
  isValid: (value: unknown) => value is T,
): readonly [T | undefined, (value: T) => void, () => void] {
  return useSessionDraft(`${PREFIX}${formId}`, isValid);
}
