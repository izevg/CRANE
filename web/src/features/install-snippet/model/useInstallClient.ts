import { useCallback, useState } from "react";
import type { Format, InstallClient } from "~/shared/api";
import { readStored, writeStored } from "~/shared/lib/state";
import { isInstallClient, storageKey } from "./clients";

type Choice = { format: Format; client: InstallClient };

/**
 * Выбранный клиент пакетов для формата — для СТРАНИЦЫ: она запрашивает инструкции с этим клиентом и передаёт его
 * блоку установки. Источники по порядку: явный выбор в этой сессии, сохранённый в `localStorage`.
 *
 * До первого ответа сервера списка клиентов нет (`available` не задан): хук отдаёт сохранённого клиента или
 * `undefined`, и страница делает первый запрос с ним или без `client` (сервер выбирает клиента по умолчанию).
 * Когда пришёл `available_clients`, хук сам отбрасывает клиента вне списка и берёт первого доступного. Проверка
 * живёт здесь, а не в эффекте при монтировании: это выводимое значение, и запрос страницы просто перезапустится с
 * новым ключом, без лишнего рендера и двойного вызова `onClientChange`.
 */
export function useInstallClient(
  format: Format,
  available?: readonly InstallClient[],
): readonly [InstallClient | undefined, (client: InstallClient) => void] {
  const [choice, setChoice] = useState<Choice | null>(null);
  const setClient = useCallback(
    (client: InstallClient) => {
      writeStored("local", storageKey(format), client);
      setChoice({ format, client });
    },
    [format],
  );
  const allowed = (value: InstallClient | undefined): InstallClient | undefined =>
    value !== undefined && (available === undefined || available.includes(value))
      ? value
      : undefined;
  const explicit = choice?.format === format ? allowed(choice.client) : undefined;
  const saved = allowed(readStored("local", storageKey(format), isInstallClient));
  return [explicit ?? saved ?? available?.[0], setClient];
}
