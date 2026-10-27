import { useLingui } from "@lingui/react/macro";
import { ConfirmDialog } from "~/shared/ui/dialogs";

type StepUpDialogProps = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Название действия, ради которого просят подтвердить вход: «Раскрыть данные события». */
  actionLabel: string;
  /** Переход в Keycloak выполняет вызывающий: компонент не знает ни роутера, ни адреса. */
  onContinue: () => void;
};

/**
 * Подтверждение входа вторым фактором (patterns §21). Своих полей кода нет: подтверждение проходит в Keycloak
 * (WCAG 2.2 3.3.8), диалог только объясняет, что произойдёт, и ведёт туда. Введённое в форме сохраняется на время
 * редиректа: черновик ведёт вызывающий через `useSessionDraft`.
 */
export function StepUpDialog({ open, onOpenChange, actionLabel, onContinue }: StepUpDialogProps) {
  const { t } = useLingui();
  return (
    <ConfirmDialog
      open={open}
      onOpenChange={onOpenChange}
      title={t`Подтвердите вход`}
      description={t`Действие «${actionLabel}» требует подтверждения вторым фактором`}
      note={t`После подтверждения вы вернётесь к действию, данные формы сохранятся`}
      size="wide"
      confirm={{ label: t`Продолжить в Keycloak`, run: { onSelect: onContinue } }}
    />
  );
}
