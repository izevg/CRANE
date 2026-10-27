import type { InstallClient } from "~/shared/api";

export type Substitution = { requested: InstallClient; shown: InstallClient };

/**
 * Подмена клиента сервером: пользователь предпочёл одного, а инструкции пришли для другого (клиент несовместим с
 * репозиторием, сервер дал клиента по умолчанию). Подмену нельзя делать молча, блок объясняет её заметкой. Пока идёт
 * запрос, расхождение — не подмена, а ожидание ответа.
 */
export function substitution(
  preferred: InstallClient | undefined,
  actual: InstallClient | undefined,
  loading: boolean,
): Substitution | null {
  if (loading || preferred === undefined || actual === undefined || preferred === actual) {
    return null;
  }
  return { requested: preferred, shown: actual };
}
