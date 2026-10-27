import { useCallback, useId, useState } from "react";
import { type Action, ActionButton } from "../../actions";
import { CodeBlock } from "../../data-display";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  Callout,
  CalloutTitle,
  Checkbox,
} from "../../primitives";

const WARNING = "Сохраните сейчас — больше мы его не покажем";
const COPY = "Копировать токен";
const SAVED = "Я сохранил токен";
const DONE = "Готово";
const DONE_REASON = "Отметьте, что сохранили токен";

type SecretOnceDialogProps = {
  open: boolean;
  /** Секрет живёт только в памяти диалога: при `open=false` его нет в DOM. */
  secret: string;
  title: string;
  /** Пользователь сохранил секрет и закрывает диалог. */
  onDone: () => void;
};

type BodyProps = Omit<SecretOnceDialogProps, "open">;

function SecretDialogBody({ secret, title, onDone }: BodyProps) {
  const [saved, setSaved] = useState(false);
  const checkboxId = useId();
  const handleOpenChange = useCallback(
    (next: boolean, details: { cancel: () => void }) => {
      // Esc закрывает диалог только после отметки; клик вне окна AlertDialog игнорирует сам
      if (next) {
        return;
      }
      if (saved) {
        onDone();
      } else {
        details.cancel();
      }
    },
    [saved, onDone],
  );
  const done: Action = {
    label: DONE,
    run: { onSelect: onDone },
    ...(saved ? {} : { disabledReason: DONE_REASON }),
  };
  return (
    <AlertDialog open onOpenChange={handleOpenChange}>
      <AlertDialogContent size="wide">
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
        </AlertDialogHeader>
        <Callout tone="warning">
          <CalloutTitle>{WARNING}</CalloutTitle>
        </Callout>
        <CodeBlock code={secret} copyLabel={COPY} />
        <div className="flex items-center gap-2 text-body">
          <Checkbox id={checkboxId} checked={saved} onCheckedChange={setSaved} />
          <label htmlFor={checkboxId}>{SAVED}</label>
        </div>
        <AlertDialogFooter>
          <ActionButton action={done} variant="primary" />
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

/**
 * Показ секрета один раз (токен, ключ вебхука). «Готово» недоступна до отметки, Esc и клик вне окна не закрывают
 * диалог до неё. Состояние и секрет существуют, только пока диалог открыт: закрытый диалог не рендерит ничего.
 */
export function SecretOnceDialog({ open, ...body }: SecretOnceDialogProps) {
  return open ? <SecretDialogBody {...body} /> : null;
}
