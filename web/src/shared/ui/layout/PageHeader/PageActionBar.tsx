import { ActionButton, ActionMenu } from "../../actions";
import { hasPageActions, type PageActions } from "./pageActions";

/** Нет действий — нет блока: пустой контейнер не занимает место и не создаёт лишнего отступа. */
export function PageActionBar(actions: PageActions) {
  const { primary, secondary, menu } = actions;
  if (!hasPageActions(actions)) {
    return null;
  }
  return (
    <div data-slot="page-actions" className="flex flex-wrap items-center gap-2">
      {primary ? <ActionButton action={primary} variant="primary" /> : null}
      {secondary?.map((action) => (
        <ActionButton key={action.label} action={action} variant="secondary" />
      ))}
      {menu ? <ActionMenu actions={menu} /> : null}
    </div>
  );
}
