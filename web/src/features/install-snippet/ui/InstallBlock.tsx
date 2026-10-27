import { useLingui } from "@lingui/react/macro";
import { ChevronDownIcon } from "lucide-react";
import { useCallback, useEffect, useId, useState } from "react";
import type {
  CodeBlock as CodeSnippet,
  Format,
  InstallClient,
  InstallInstructions,
} from "~/shared/api";
import { CodeBlock } from "~/shared/ui/data-display";
import { SegmentedControl } from "~/shared/ui/inputs";
import { Button, Card, CardContent, CardHeader, Link } from "~/shared/ui/primitives";
import { clientLabel } from "../model/clients";
import { useInstallClient } from "../model/useInstallClient";

type InstallBlockProps = {
  format: Format;
  /** Клиенты, доступные для формата (`CLIENTS_BY_FORMAT[format]`). */
  clients: readonly InstallClient[];
  /** Инструкция сервера для выбранного клиента; токена в ней нет, только `${CRANE_TOKEN}`. */
  instructions: Pick<InstallInstructions, "client" | "setup_steps" | "manual_config"> & {
    install: CodeSnippet;
  };
  /**
   * Выбранный клиент отличается от клиента показанной инструкции (пользователь переключил или подхватился
   * сохранённый выбор): страница запрашивает инструкцию для него.
   */
  onClientChange: (client: InstallClient) => void;
  /** Маршрут страницы «Переменные окружения» (SPA-ссылка, а не `href` из API). */
  envHref: string;
};

/**
 * Блок «Установка» (patterns §7): клиент пакетов, шаг 1 с настройкой один раз, ручная настройка файлом и шаг 2
 * с командой установки. Выбор клиента помнится по формату. Данные приходят готовыми, блок их не запрашивает.
 */
export function InstallBlock({
  format,
  clients,
  instructions,
  onClientChange,
  envHref,
}: InstallBlockProps) {
  const { t } = useLingui();
  const [client, setClient] = useInstallClient(format, clients);
  const [manualOpen, setManualOpen] = useState(false);
  const manualId = useId();
  const toggleManual = useCallback(() => setManualOpen((open) => !open), []);
  const shown = instructions.client;
  useEffect(() => {
    if (client !== undefined && client !== shown) {
      onClientChange(client);
    }
  }, [client, shown, onClientChange]);
  const items = clients.map((value) => ({ value, label: clientLabel(value) }));
  const { path, code } = instructions.manual_config;
  return (
    <Card>
      <CardHeader>
        <h2 className="text-h3">{t`Установка`}</h2>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {client === undefined ? null : (
          <SegmentedControl
            label={t`Клиент пакетов`}
            items={items}
            value={client}
            onValueChange={setClient}
          />
        )}
        <div className="flex flex-col gap-2">
          <h3 className="text-body font-semibold">{t`Шаг 1. Однократная настройка`}</h3>
          {instructions.setup_steps.map((step) => (
            <CodeBlock
              key={step.title}
              code={step.code}
              label={step.title}
              copyLabel={t`Копировать команду настройки`}
            />
          ))}
          <Button
            variant="ghost"
            size="sm"
            aria-expanded={manualOpen}
            aria-controls={manualId}
            onClick={toggleManual}
            className="self-start"
          >
            <ChevronDownIcon aria-hidden="true" className={manualOpen ? "rotate-180" : ""} />
            {t`Настроить вручную (${path})`}
          </Button>
          <div id={manualId} hidden={!manualOpen} className="flex flex-col gap-2">
            <CodeBlock code={code} label={t`Файл ${path}`} copyLabel={t`Копировать ${path}`} />
            <p className="text-caption text-muted-foreground">
              {t`Токен не показывается: используйте cctl или переменную окружения.`}{" "}
              <Link href={envHref} className="underline underline-offset-3">
                {t`Переменные окружения`}
              </Link>
            </p>
          </div>
        </div>
        <div className="flex flex-col gap-2">
          <h3 className="text-body font-semibold">{t`Шаг 2. Установка`}</h3>
          <CodeBlock
            code={instructions.install.code}
            label={t`Команда установки`}
            copyLabel={t`Копировать команду установки`}
          />
        </div>
      </CardContent>
    </Card>
  );
}
