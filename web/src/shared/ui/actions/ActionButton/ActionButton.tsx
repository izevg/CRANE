import { useCallback, useId } from "react";
import {
  Badge,
  Button,
  type ButtonVariant,
  buttonStyles,
  Link,
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "../../primitives";
import type { Action } from "./action";

const MFA = "MFA";
const STEP_UP_HINT = ", требуется подтверждение входа";

type ActionButtonVariant = "primary" | "secondary" | "ghost";

/** Подпись действия: у step-up добавляется бейдж «MFA», а скринридер получает пояснение. */
export function ActionContent({ action }: { action: Action }) {
  return (
    <>
      {action.label}
      {action.stepUp ? (
        <>
          <Badge variant="inherit">{MFA}</Badge>
          <span className="sr-only">{STEP_UP_HINT}</span>
        </>
      ) : null}
    </>
  );
}

type Props = {
  action: Action;
  variant: ActionButtonVariant;
};

/**
 * Кнопка действия. Отсутствие действия — отсутствие кнопки: вызывающий не рендерит компонент без `action`.
 * Недоступное действие (`disabledReason`) остаётся фокусируемым (`aria-disabled`), чтобы причину можно было
 * прочитать с клавиатуры; нажатие ничего не делает.
 */
export function ActionButton({ action, variant }: Props) {
  const reasonId = useId();
  const blocked = action.disabledReason !== undefined;
  const style: ButtonVariant = action.destructive ? "destructive" : variant;
  const content = <ActionContent action={action} />;
  const { run } = action;
  const handleClick = useCallback(() => {
    if (!blocked && "onSelect" in run) {
      run.onSelect();
    }
  }, [blocked, run]);

  if (!blocked && "href" in action.run) {
    return (
      <Link href={action.run.href} className={buttonStyles({ variant: style })}>
        {content}
      </Link>
    );
  }

  const button = (
    <Button
      variant={style}
      aria-disabled={blocked || undefined}
      aria-describedby={blocked ? reasonId : undefined}
      onClick={handleClick}
    >
      {content}
    </Button>
  );

  if (action.disabledReason === undefined) {
    return button;
  }
  return (
    <>
      <Tooltip>
        <TooltipTrigger render={button} />
        <TooltipContent>{action.disabledReason}</TooltipContent>
      </Tooltip>
      <span id={reasonId} className="sr-only">
        {action.disabledReason}
      </span>
    </>
  );
}
