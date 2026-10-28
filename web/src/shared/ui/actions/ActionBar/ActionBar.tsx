import { cx } from "~/shared/lib/core";
import { ActionButton } from "../ActionButton/ActionButton";
import { ActionMenu } from "../ActionButton/ActionMenu";
import type { Action, ActionGroup } from "../ActionButton/action";

type ActionBarProps = {
  /**
   * `ActionGroup` — primary + secondary + меню. Плоский массив — ряд одинаковых secondary-кнопок любой длины
   * (массовые действия, где главного действия нет).
   */
  actions: ActionGroup | readonly Action[];
  /** Только размещение (отступ, выравнивание) у внешнего узла. */
  className?: string;
};

function isFlat(actions: ActionBarProps["actions"]): actions is readonly Action[] {
  return Array.isArray(actions);
}

function isEmpty({ primary, secondary, menu }: ActionGroup): boolean {
  return primary === undefined && (secondary?.length ?? 0) === 0 && (menu?.length ?? 0) === 0;
}

/**
 * Единственное место, где действия превращаются в кнопки. Нет `Action` — нет кнопки;
 * группа без действий не рисует даже пустой контейнер.
 */
export function ActionBar({ actions, className }: ActionBarProps) {
  const root = cx("flex flex-wrap items-center gap-2", className);
  if (isFlat(actions)) {
    if (actions.length === 0) {
      return null;
    }
    return (
      <div data-slot="action-bar" className={root}>
        {actions.map((action) => (
          <ActionButton key={action.label} action={action} variant="secondary" />
        ))}
      </div>
    );
  }
  if (isEmpty(actions)) {
    return null;
  }
  const { primary, secondary, menu } = actions;
  return (
    <div data-slot="action-bar" className={root}>
      {primary ? <ActionButton action={primary} variant="primary" /> : null}
      {secondary?.map((action) => (
        <ActionButton key={action.label} action={action} variant="secondary" />
      ))}
      {menu ? <ActionMenu actions={menu} /> : null}
    </div>
  );
}
