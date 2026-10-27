import { useLingui } from "@lingui/react/macro";
import { ChevronDownIcon } from "lucide-react";
import { useCallback, useId, useState } from "react";
import type {
  CodeBlock as CodeSnippet,
  Format,
  InstallClient,
  InstallInstructions,
} from "~/shared/api";
import { CodeBlock } from "~/shared/ui/data-display";
import { SegmentedControl } from "~/shared/ui/inputs";
import { Button, Card, CardContent, CardHeader, Link, Skeleton } from "~/shared/ui/primitives";
import { CLIENTS_BY_FORMAT, clientLabel } from "../model/clients";

type InstallBlockProps = {
  format: Format;
  /** Выбранный клиент: его держит страница (`useInstallClient`) и по нему запрашивает инструкции. */
  client: InstallClient;
  onClientChange: (client: InstallClient) => void;
  /** Инструкция сервера; токена в ней нет, только `${CRANE_TOKEN}`. */
  instructions: Pick<InstallInstructions, "setup_steps" | "manual_config"> & {
    install: CodeSnippet;
  };
  /**
   * Инструкции пока не для выбранного клиента (идёт запрос): вместо старых команд — заглушки, копировать нечего,
   * чтобы нельзя было взять команду чужого клиента.
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
 * с командой установки. Блок управляемый: клиента и данные дают пропсы, список клиентов берётся из формата.
 */
export function InstallBlock({
  format,
  client,
  onClientChange,
  instructions,
  loading = false,
  envHref,
}: InstallBlockProps) {
  const { t } = useLingui();
  const [manualOpen, setManualOpen] = useState(false);
  const manualId = useId();
  const toggleManual = useCallback(() => setManualOpen((open) => !open), []);
  const items = CLIENTS_BY_FORMAT[format].map((value) => ({
    value,
    label: clientLabel(value),
  }));
  const { path, code } = instructions.manual_config;
  return (
    <Card>
      <CardHeader>
        <h2 className="text-h3">{t`Установка`}</h2>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">
        <SegmentedControl
          label={t`Клиент пакетов`}
          items={items}
          value={client}
          onValueChange={onClientChange}
        />
        <div className="flex flex-col gap-2" aria-busy={loading}>
          <h3 className="text-body font-semibold">{t`Шаг 1. Однократная настройка`}</h3>
          {loading ? (
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
            {t`Настроить вручную (${path})`}
          </Button>
          <div id={manualId} hidden={!manualOpen} className="flex flex-col gap-2">
            {loading ? (
              <CommandSkeleton />
            ) : (
              <CodeBlock code={code} label={t`Файл ${path}`} copyLabel={t`Копировать ${path}`} />
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
          {loading ? (
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
