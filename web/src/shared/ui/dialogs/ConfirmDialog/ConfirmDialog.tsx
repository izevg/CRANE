import { type ReactNode, useCallback, useRef } from "react";
import { type Action, ActionButton } from "../../actions";
import {
  AlertDialog,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "../../primitives";

const CANCEL = "Отмена";
const PENDING_REASON = "Выполняется…";
const FIELD = "input,textarea,select,[role=combobox]";
const CONFIRM_SLOT = "[data-confirm-slot] :is(button,a)";

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
  const cancelRef = useRef<HTMLButtonElement>(null);
  const popupRef = useRef<HTMLDivElement>(null);
  const firstFocus = useCallback(() => {
    const popup = popupRef.current;
    return (
      popup?.querySelector<HTMLElement>(FIELD) ??
      popup?.querySelector<HTMLElement>(CONFIRM_SLOT) ??
      null
    );
  }, []);
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
      <AlertDialogContent
        ref={popupRef}
        initialFocus={confirm.destructive ? cancelRef : firstFocus}
      >
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          {description === undefined ? null : (
            <AlertDialogDescription>{description}</AlertDialogDescription>
          )}
        </AlertDialogHeader>
        {consequences && consequences.length > 0 ? (
          <ul className="m-0 ml-5 flex list-disc flex-col gap-1 text-body">
            {consequences.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        ) : null}
        {children}
        <AlertDialogFooter>
          <AlertDialogCancel ref={cancelRef}>{CANCEL}</AlertDialogCancel>
          <span data-confirm-slot="" className="contents">
            <ActionButton action={action} variant="primary" />
          </span>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
