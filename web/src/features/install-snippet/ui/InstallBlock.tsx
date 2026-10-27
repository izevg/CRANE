import { useLingui } from "@lingui/react/macro";
import { ChevronDownIcon } from "lucide-react";
import { useCallback, useId, useState } from "react";
import type { InstallClient, InstallInstructions } from "~/shared/api";
import { CodeBlock } from "~/shared/ui/data-display";
import { SegmentedControl } from "~/shared/ui/inputs";
import { Button, Card, CardContent, CardHeader, Link, Skeleton } from "~/shared/ui/primitives";
import { clientLabel } from "../model/clients";
import { substitution } from "../model/substitution";

type InstallBlockProps = {
  /** Предпочтение пользователя (`useInstallClient`); `undefined`, пока он ничего не выбирал. */
  preferred: InstallClient | undefined;
  onClientChange: (client: InstallClient) => void;
  /**
   * Инструкция сервера (токена в ней нет, только `${CRANE_TOKEN}`); `undefined`, пока ответа ещё не было. Клиент
   * в ней — фактический: его выделяет сегмент, а список — `available_clients`.
   */
  instructions:
    | Pick<
        InstallInstructions,
        "client" | "available_clients" | "setup_steps" | "manual_config" | "install"
      >
    | undefined;
  /**
   * Идёт запрос инструкций (например, после смены клиента): вместо старых команд — заглушки, взять команду чужого
   * клиента нельзя.
   */
  loading?: boolean;
  /** Маршрут страницы «Переменные окружения» (SPA-ссылка, а не `href` из API). */
  envHref: string;
};

function CommandSkeleton() {
  return <Skeleton className="h-16 w-full" />;
}

/**
 * Блок «Установка» (patterns §7): клиент пакетов, шаг 1 с настройкой один раз, ручная настройка файлом и шаг 2
 * с командой установки. Блок управляемый: данные и состояние запроса дают пропсы. Если сервер подменил
 * предпочитаемого клиента, под сегментом появляется заметка.
 */
export function InstallBlock({
  preferred,
  onClientChange,
  instructions,
  loading = false,
  envHref,
}: InstallBlockProps) {
  const { t } = useLingui();
  const [manualOpen, setManualOpen] = useState(false);
  const manualId = useId();
  const toggleManual = useCallback(() => setManualOpen((open) => !open), []);
  // Инструкция, которую можно показывать и копировать: ответ получен и не перезапрашивается
  const current = loading ? undefined : instructions;
  const items = (instructions?.available_clients ?? []).map((value) => ({
    value,
    label: clientLabel(value),
  }));
  // Пока ждём ответ, сегмент показывает выбор пользователя; после — клиента, которого дал сервер
  const selected =
    (loading ? preferred : instructions?.client) ?? instructions?.client ?? preferred;
  const swapped = substitution(preferred, instructions?.client, loading);
  const requestedLabel = swapped === null ? "" : clientLabel(swapped.requested);
  const shownLabel = swapped === null ? "" : clientLabel(swapped.shown);
  const path = current?.manual_config.path;
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
        {swapped === null ? null : (
          <p role="status" className="text-caption text-muted-foreground">
            {t`${requestedLabel} недоступен для этого репозитория, показаны команды для ${shownLabel}`}
          </p>
        )}
        <div className="flex flex-col gap-2" aria-busy={current === undefined}>
          <h3 className="text-body font-semibold">{t`Шаг 1. Однократная настройка`}</h3>
          {current === undefined ? (
            <CommandSkeleton />
          ) : (
            current.setup_steps.map(({ title, code: stepCode }) => (
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
            {path === undefined ? t`Настроить вручную` : t`Настроить вручную (${path})`}
          </Button>
          <div id={manualId} hidden={!manualOpen} className="flex flex-col gap-2">
            {current === undefined ? (
              <CommandSkeleton />
            ) : (
              <CodeBlock
                code={current.manual_config.code}
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
        <div className="flex flex-col gap-2" aria-busy={current === undefined}>
          <h3 className="text-body font-semibold">{t`Шаг 2. Установка`}</h3>
          {current === undefined ? (
            <CommandSkeleton />
          ) : (
            <CodeBlock
              code={current.install.code}
              label={t`Команда установки`}
              copyLabel={t`Копировать команду установки`}
            />
          )}
        </div>
      </CardContent>
    </Card>
  );
}
