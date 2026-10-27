import { useCallback, useState } from "react";
import type { Format, InstallClient } from "~/shared/api";
import { readStored, writeStored } from "~/shared/lib/state";
import { CLIENTS_BY_FORMAT, storageKey } from "./clients";

type Choice = { format: Format; client: InstallClient };

/**
 * Выбранный клиент пакетов для формата — для СТРАНИЦЫ: она сразу запрашивает инструкции с этим клиентом и передаёт
 * его блоку установки. Источники по порядку: явный выбор в этой сессии, сохранённый в `localStorage` (если он всё
 * ещё доступен для формата), первый доступный. Выбор пишется в хранилище и живёт отдельно у каждого формата.
 */
export function useInstallClient(
  format: Format,
): readonly [InstallClient, (client: InstallClient) => void] {
  const available: readonly [InstallClient, ...InstallClient[]] = CLIENTS_BY_FORMAT[format];
  const [choice, setChoice] = useState<Choice | null>(null);
  const setClient = useCallback(
    (client: InstallClient) => {
      writeStored("local", storageKey(format), client);
      setChoice({ format, client });
    },
    [format],
  );
  const isAvailable = (value: unknown): value is InstallClient =>
    available.some((client) => client === value);
  const explicit =
    choice?.format === format && isAvailable(choice.client) ? choice.client : undefined;
  const client = explicit ?? readStored("local", storageKey(format), isAvailable) ?? available[0];
  return [client, setClient];
}
