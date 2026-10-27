import { useCallback, useState } from "react";
import type { Format, InstallClient } from "~/shared/api";
import { readStored, writeStored } from "~/shared/lib/state";
import { isInstallClient, storageKey } from "./clients";

type Choice = { format: Format; client: InstallClient };

/**
 * Предпочитаемый клиент пакетов для формата — для СТРАНИЦЫ: она запрашивает инструкции с этим клиентом (или без
 * `client`, если предпочтения нет, и сервер выберет сам). Хук хранит только предпочтение: источники по порядку —
 * явный выбор в этой сессии и сохранённый в `localStorage` (валидируется `isInstallClient`). Какого клиента на
 * самом деле показывать, решает сервер через `instructions.client`: несовместимого с форматом он подменяет.
 */
export function useInstallClient(
  format: Format,
): readonly [InstallClient | undefined, (client: InstallClient) => void] {
  const [choice, setChoice] = useState<Choice | null>(null);
  const setPreferred = useCallback(
    (client: InstallClient) => {
      writeStored("local", storageKey(format), client);
      setChoice({ format, client });
    },
    [format],
  );
  const preferred =
    choice?.format === format
      ? choice.client
      : readStored("local", storageKey(format), isInstallClient);
  return [preferred, setPreferred];
}
