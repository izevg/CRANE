import type { Action } from "../../actions";

/**
 * Действия страницы или панели: одна primary, не больше двух secondary, остальное в меню «…».
 * Больше двух secondary тип не выражает: лишнее уходит в `menu`.
 */
export type PageActions = {
  primary?: Action;
  secondary?: readonly [Action] | readonly [Action, Action];
  menu?: readonly Action[];
};

export function hasPageActions({ primary, secondary, menu }: PageActions): boolean {
  return primary !== undefined || (secondary?.length ?? 0) > 0 || (menu?.length ?? 0) > 0;
}
