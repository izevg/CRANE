import { useLingui } from "@lingui/react/macro";
import { CopyableId } from "~/shared/ui/actions";

export function RuleLine({ ruleId }: { ruleId: string }) {
  const { t } = useLingui();
  return <p className="mt-2 text-body wrap-anywhere">{t`Правило: ${ruleId}`}</p>;
}

export function RequestIdLine({ requestId }: { requestId: string }) {
  const { t } = useLingui();
  return (
    <p className="mt-3 flex max-w-full items-center gap-1 text-caption text-muted-foreground">
      <span>{t`Идентификатор запроса:`}</span>
      <CopyableId value={requestId} label={t`Скопировать идентификатор запроса`} />
    </p>
  );
}

export function ContactLine({ contact }: { contact: string }) {
  const { t } = useLingui();
  return (
    <p className="mt-2 text-body text-muted-foreground wrap-anywhere">{t`Контакт: ${contact}`}</p>
  );
}

/** Нумерованные шаги; пустой список не рисуется вовсе. */
export function RemediationList({ steps }: { steps: readonly string[] }) {
  if (steps.length === 0) {
    return null;
  }
  return (
    <ol className="mt-4 max-w-[48ch] list-decimal space-y-1 pl-5 text-left text-body">
      {steps.map((step) => (
        <li key={step} className="wrap-anywhere">
          {step}
        </li>
      ))}
    </ol>
  );
}
