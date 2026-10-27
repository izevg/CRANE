import { type ReactNode, useCallback, useId, useRef } from "react";
import { type Action, ActionButton } from "../../actions";
import {
  AlertDialog,
  AlertDialogBody,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "../../primitives";

const CANCEL = "Отмена";
const PENDING_REASON = "Выполняется…";
const CONFIRM_CONTROL = "button,a";

type ConfirmDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Вопрос: «Удалить версию 1.4.2?». */
  title: string;
  description?: string;
  /** Что произойдёт после подтверждения (3–4 пункта). */
  consequences?: readonly string[];
  /** Поле причины и подобное: `Field` из примитивов собирает вызывающий. */
  children?: ReactNode;
  /** Подтверждающее действие. `destructive` переводит начальный фокус на «Отмена». */
  confirm: Action;
  /** Действие выполняется: подтверждение недоступно, диалог не закрывается по Esc. */
  pending?: boolean;
};

/**
 * Подтверждение опасного действия. Начальный фокус: у деструктивного действия на «Отмена», иначе на первое поле
 * формы или на кнопку подтверждения. Закрытие диалога после успеха остаётся за вызывающим (`onOpenChange`).
 */
export function ConfirmDialog({
  open,
  onOpenChange,
  title,
  description,
  consequences,
  children,
  confirm,
  pending = false,
}: ConfirmDialogProps) {
  const hasChildren = children !== undefined && children !== null && children !== false;
  const cancelRef = useRef<HTMLButtonElement>(null);
  const confirmRef = useRef<HTMLSpanElement>(null);
  const reasonId = useId();
  // Поля формы стоят в разметке раньше подвала, поэтому при `children` подходит поведение по умолчанию
  // (первый фокусируемый элемент). Без полей фокус идёт на кнопку подтверждения, а не на «Отмена».
  const firstFocus = useCallback(
    () =>
      hasChildren
        ? true
        : (confirmRef.current?.querySelector<HTMLElement>(CONFIRM_CONTROL) ?? true),
    [hasChildren],
  );
  const handleOpenChange = useCallback(
    (next: boolean, details: { cancel: () => void }) => {
      if (!next && pending) {
        details.cancel();
        return;
      }
      onOpenChange(next);
    },
    [pending, onOpenChange],
  );
  const action: Action = pending ? { ...confirm, disabledReason: PENDING_REASON } : confirm;
  return (
    <AlertDialog open={open} onOpenChange={handleOpenChange}>
      <AlertDialogContent initialFocus={confirm.destructive ? cancelRef : firstFocus}>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          {description === undefined ? null : (
            <AlertDialogDescription>{description}</AlertDialogDescription>
          )}
        </AlertDialogHeader>
        <AlertDialogBody>
          {consequences && consequences.length > 0 ? (
            <ul className="m-0 ml-5 flex list-disc flex-col gap-1 text-body">
              {consequences.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          ) : null}
          {children}
        </AlertDialogBody>
        <AlertDialogFooter>
          {/* Во время выполнения «Отмена» остаётся в Tab, но не закрывает диалог и объясняет почему */}
          <AlertDialogCancel
            ref={cancelRef}
            aria-disabled={pending || undefined}
            aria-describedby={pending ? reasonId : undefined}
          >
            {CANCEL}
          </AlertDialogCancel>
          {pending ? (
            <span id={reasonId} className="sr-only">
              {PENDING_REASON}
            </span>
          ) : null}
          <span ref={confirmRef} className="contents">
            <ActionButton action={action} variant="primary" />
          </span>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
