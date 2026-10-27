// i18n: не мигрирован
import { CopyableId } from "~/shared/ui/actions";

const RULE = "Правило";
const REQUEST_ID = "Идентификатор запроса";
const COPY_REQUEST_ID = "Скопировать идентификатор запроса";
const CONTACT = "Контакт";

export function RuleLine({ ruleId }: { ruleId: string }) {
  return <p className="mt-2 text-body wrap-anywhere">{`${RULE}: ${ruleId}`}</p>;
}

export function RequestIdLine({ requestId }: { requestId: string }) {
  return (
    <p className="mt-3 flex max-w-full items-center gap-1 text-caption text-muted-foreground">
      <span>{`${REQUEST_ID}:`}</span>
      <CopyableId value={requestId} label={COPY_REQUEST_ID} />
    </p>
  );
}

export function ContactLine({ contact }: { contact: string }) {
  return (
    <p className="mt-2 text-body text-muted-foreground wrap-anywhere">{`${CONTACT}: ${contact}`}</p>
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
