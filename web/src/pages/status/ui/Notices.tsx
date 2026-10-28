import { useLingui } from "@lingui/react/macro";
import { useCallback, useEffect, useRef } from "react";
import { announce } from "~/shared/lib/a11y";
import { useTextFormat } from "~/shared/lib/i18n";
import { useNow } from "~/shared/lib/state";
import { Button, Callout, CalloutDescription, CalloutTitle } from "~/shared/ui/primitives";
import { degradedLabel } from "../model/labels";

/** «Обновлено 3 мин назад» одним сообщением: порядок слов и форма времени зависят от языка. */
export function UpdatedAt({ ts }: { ts: number }) {
  const { t } = useLingui();
  const format = useTextFormat();
  const now = useNow();
  const ago = format.relative(ts, now);
  return (
    <time
      dateTime={new Date(ts).toISOString()}
      title={format.dateTime(ts)}
      className="text-caption text-muted-foreground"
    >
      {t`Обновлено ${ago}`}
    </time>
  );
}

/**
 * Ошибка очередного опроса: последние данные остаются на экране, повтор — по кнопке. Во время повтора кнопка занята;
 * если повтор не удался, это объявляется один раз (текст сообщения при этом не меняется, и без объявления
 * пользователь не заметил бы результата).
 */
export function RefreshFailed({
  onRetry,
  retrying,
  failedAt,
}: {
  onRetry: () => void;
  retrying: boolean;
  /** Время последней ошибки: меняется при каждой новой неудаче, даже мгновенной. */
  failedAt: number;
}) {
  const { t } = useLingui();
  const asked = useRef<boolean>(false);
  const failure = t`Не удалось обновить данные. Попробуйте ещё раз.`;
  // Срабатывает на новую ошибку, а не на смену `retrying`: быстрый сбой мог не успеть отрисоваться занятой кнопкой
  useEffect(() => {
    if (asked.current && failedAt > 0) {
      asked.current = false;
      announce(failure);
    }
  }, [failedAt, failure]);
  const retry = useCallback(() => {
    asked.current = true;
    onRetry();
  }, [onRetry]);
  return (
    <Callout tone="warning">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span>{t`Не удалось обновить данные. Показаны последние полученные.`}</span>
        <Button variant="secondary" size="sm" pending={retrying} onClick={retry}>
          {t`Повторить`}
        </Button>
      </div>
    </Callout>
  );
}

/** Деградации платформы одним блоком; пустой список не рисуется. */
export function DegradedCallout({ codes }: { codes: readonly string[] }) {
  const { t } = useLingui();
  if (codes.length === 0) {
    return null;
  }
  return (
    <Callout tone="warning">
      <CalloutTitle>{t`Часть возможностей работает с ограничениями`}</CalloutTitle>
      <CalloutDescription>
        <ul className="list-disc pl-5">
          {codes.map((code) => (
            <li key={code}>{degradedLabel(code)}</li>
          ))}
        </ul>
      </CalloutDescription>
    </Callout>
  );
}
