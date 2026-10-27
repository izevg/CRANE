import { useCallback, useState } from "react";
import type { Format, InstallClient } from "~/shared/api";
import { readStored, writeStored } from "~/shared/lib/state";
import { storageKey } from "./clients";

type Choice = { format: Format; client: InstallClient };

/** Значение подходит, только если это один из клиентов, доступных сейчас: чужой или устаревший выбор отбрасывается. */
function availableGuard(available: readonly InstallClient[]) {
  return (value: unknown): value is InstallClient => available.some((client) => client === value);
}

/**
 * Выбранный клиент пакетов для формата. Источники по порядку: явный выбор в этой сессии, сохранённый в
 * `localStorage` (если он всё ещё доступен для формата), первый доступный. Выбор пишется в хранилище и живёт
 * отдельно у каждого формата.
 */
export function useInstallClient(
  format: Format,
  available: readonly InstallClient[],
): readonly [InstallClient | undefined, (client: InstallClient) => void] {
  const [choice, setChoice] = useState<Choice | null>(null);
  const setClient = useCallback(
    (client: InstallClient) => {
      writeStored("local", storageKey(format), client);
      setChoice({ format, client });
    },
    [format],
  );
  const guard = availableGuard(available);
  const explicit = choice?.format === format && guard(choice.client) ? choice.client : undefined;
  const client = explicit ?? readStored("local", storageKey(format), guard) ?? available[0];
  return [client, setClient];
}
