import { useLingui } from "@lingui/react/macro";
import { Button, Callout, CalloutDescription, CalloutTitle } from "~/shared/ui/primitives";
import { degradedLabel } from "../model/labels";

/** Ошибка очередного опроса: последние данные остаются на экране, повтор — по кнопке. */
export function RefreshFailed({ onRetry }: { onRetry: () => void }) {
  const { t } = useLingui();
  return (
    <Callout tone="warning">
      <div className="flex flex-wrap items-center justify-between gap-2">
        <span>{t`Не удалось обновить данные. Показаны последние полученные`}</span>
        <Button variant="secondary" size="sm" onClick={onRetry}>
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
