/** Suspense-запрос считается устаревшим не раньше секунды: меньше TanStack Query ему не позволяет. */
const SUSPENSE_STALE_MS = 1100;

/**
 * Имитирует возврат на вкладку: устаревшие активные запросы перечитываются (так же, как при опросе). Истории
 * «обновление не удалось» и «401 в фоне» строятся на втором ответе сервера.
 */
export async function refetchStale(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, SUSPENSE_STALE_MS));
  window.dispatchEvent(new Event("visibilitychange"));
}
