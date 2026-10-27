// i18n: не мигрирован
import { useCallback, useId, useState } from "react";
import { type Action, ActionButton } from "../../actions";
import { CodeBlock } from "../../data-display";
import {
  AlertDialog,
  AlertDialogBody,
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
  /** Секрет живёт только в памяти диалога: `null` — диалог закрыт и секрета в DOM нет. */
  secret: string | null;
  title: string;
  /** Пользователь сохранил секрет и закрывает диалог. */
  onDone: () => void;
};

type BodyProps = Omit<SecretOnceDialogProps, "secret"> & { secret: string };

function SecretDialogBody({ secret, title, onDone }: BodyProps) {
  const [saved, setSaved] = useState(false);
  const checkboxId = useId();
  // Диалог открыт, пока есть секрет, поэтому клик вне окна и Esc до отметки ничего не закрывают сами:
  // достаточно не вызывать onDone. После отметки Esc завершает показ.
  const handleOpenChange = useCallback(
    (next: boolean) => {
      if (!next && saved) {
        onDone();
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
        <AlertDialogBody className="flex flex-col gap-4">
          <Callout tone="warning">
            <CalloutTitle>{WARNING}</CalloutTitle>
          </Callout>
          <CodeBlock code={secret} copyLabel={COPY} machine />
          <div className="flex items-center gap-2 text-body">
            <Checkbox id={checkboxId} checked={saved} onCheckedChange={setSaved} />
            <label htmlFor={checkboxId}>{SAVED}</label>
          </div>
        </AlertDialogBody>
        <AlertDialogFooter>
          <ActionButton action={done} variant="primary" />
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

/**
 * Показ секрета один раз (токен, ключ вебхука). «Готово» недоступна до отметки, Esc и клик вне окна не закрывают
 * диалог до неё. Состояние и секрет существуют, только пока `secret` не `null`.
 */
export function SecretOnceDialog({ secret, ...rest }: SecretOnceDialogProps) {
  return secret === null ? null : <SecretDialogBody secret={secret} {...rest} />;
}
