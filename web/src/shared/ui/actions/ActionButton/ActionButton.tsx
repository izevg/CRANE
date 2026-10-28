import { useLingui } from "@lingui/react/macro";
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

type ActionButtonVariant = "primary" | "secondary" | "ghost";
type ChipVariant = "inherit" | "inverse" | "control";

/** Подпись действия: у step-up добавляется бейдж «MFA», а скринридер получает пояснение. */
export function ActionContent({
  action,
  chip = "inherit",
}: {
  action: Action;
  /** Поверхность бейджа: на кнопке она своя (`inverse`/`control`), в пункте меню — цвет пункта. */
  chip?: ChipVariant;
}) {
  const { t } = useLingui();
  return (
    <>
      {action.label}
      {action.stepUp ? (
        <>
          <Badge variant={chip} className="ml-1">
            {MFA}
          </Badge>
          <span className="sr-only">{t`, требуется подтверждение входа`}</span>
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
  // Бейдж не должен сливаться с фоном кнопки: на заливке primary он инверсный, на остальных — с рамкой контрола.
  const chip: ChipVariant = style === "primary" && !blocked ? "inverse" : "control";
  const content = <ActionContent action={action} chip={chip} />;
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
