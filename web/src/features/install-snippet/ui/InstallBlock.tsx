import { useLingui } from "@lingui/react/macro";
import { ChevronDownIcon } from "lucide-react";
import { useCallback, useId, useState } from "react";
import type { InstallClient, InstallInstructions } from "~/shared/api";
import { CodeBlock } from "~/shared/ui/data-display";
import { SegmentedControl } from "~/shared/ui/inputs";
import { Button, Card, CardContent, CardHeader, Link, Skeleton } from "~/shared/ui/primitives";
import { clientLabel } from "../model/clients";

type InstallBlockProps = {
  /** Выбранный клиент: его держит страница (`useInstallClient`); `undefined` до первого ответа без сохранённого. */
  client: InstallClient | undefined;
  onClientChange: (client: InstallClient) => void;
  /**
   * Инструкция сервера (токена в ней нет, только `${CRANE_TOKEN}`); `undefined`, пока ответа ещё не было. Список
   * клиентов — её `available_clients`. Если инструкция не для выбранного клиента (идёт повторный запрос), вместо
   * старых команд показываются заглушки: взять команду чужого клиента нельзя.
   */
  instructions:
    | Pick<
        InstallInstructions,
        "client" | "available_clients" | "setup_steps" | "manual_config" | "install"
      >
    | undefined;
  /** Маршрут страницы «Переменные окружения» (SPA-ссылка, а не `href` из API). */
  envHref: string;
};

function CommandSkeleton() {
  return <Skeleton className="h-16 w-full" />;
}

/**
 * Блок «Установка» (patterns §7): клиент пакетов, шаг 1 с настройкой один раз, ручная настройка файлом и шаг 2
 * с командой установки. Блок управляемый: клиента и данные дают пропсы, список клиентов берётся из формата.
 */
export function InstallBlock({ client, onClientChange, instructions, envHref }: InstallBlockProps) {
  const { t } = useLingui();
  const [manualOpen, setManualOpen] = useState(false);
  const manualId = useId();
  const toggleManual = useCallback(() => setManualOpen((open) => !open), []);
  const loading =
    instructions === undefined || (client !== undefined && instructions.client !== client);
  const items = (instructions?.available_clients ?? []).map((value) => ({
    value,
    label: clientLabel(value),
  }));
  const selected = client ?? instructions?.client;
  const path = instructions?.manual_config.path ?? "";
  return (
    <Card>
      <CardHeader>
        <h2 className="text-h3">{t`Установка`}</h2>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        {selected === undefined || items.length === 0 ? (
          <Skeleton className="h-8 w-48" />
        ) : (
          <SegmentedControl
            label={t`Клиент пакетов`}
            items={items}
            value={selected}
            onValueChange={onClientChange}
          />
        )}
        <div className="flex flex-col gap-2" aria-busy={loading}>
          <h3 className="text-body font-semibold">{t`Шаг 1. Однократная настройка`}</h3>
          {instructions === undefined || loading ? (
            <CommandSkeleton />
          ) : (
            instructions.setup_steps.map(({ title, code: stepCode }) => (
              <CodeBlock
                key={title}
                code={stepCode}
                label={title}
                copyLabel={t`Копировать: ${title}`}
              />
            ))
          )}
          <Button
            variant="ghost"
            size="sm"
            aria-expanded={manualOpen}
            aria-controls={manualId}
            onClick={toggleManual}
            className="self-start"
          >
            <ChevronDownIcon aria-hidden="true" className={manualOpen ? "rotate-180" : ""} />
            {path === "" ? t`Настроить вручную` : t`Настроить вручную (${path})`}
          </Button>
          <div id={manualId} hidden={!manualOpen} className="flex flex-col gap-2">
            {instructions === undefined || loading ? (
              <CommandSkeleton />
            ) : (
              <CodeBlock
                code={instructions.manual_config.code}
                label={t`Файл ${path}`}
                copyLabel={t`Копировать ${path}`}
              />
            )}
            <p className="text-caption text-muted-foreground">
              {t`Токен не показывается: используйте cctl или переменную окружения.`}{" "}
              <Link href={envHref} className="underline underline-offset-3">
                {t`Переменные окружения`}
              </Link>
            </p>
          </div>
        </div>
        <div className="flex flex-col gap-2" aria-busy={loading}>
          <h3 className="text-body font-semibold">{t`Шаг 2. Установка`}</h3>
          {instructions === undefined || loading ? (
            <CommandSkeleton />
          ) : (
            <CodeBlock
              code={instructions.install.code}
              label={t`Команда установки`}
              copyLabel={t`Копировать команду установки`}
            />
          )}
        </div>
      </CardContent>
    </Card>
  );
}
