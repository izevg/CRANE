// i18n: не мигрирован
import { MoreHorizontalIcon } from "lucide-react";
import {
  Button,
  Link,
  Menu,
  MenuContent,
  MenuItem,
  MenuSeparator,
  MenuTrigger,
} from "../../primitives";
import { ActionContent } from "./ActionButton";
import type { Action } from "./action";

const MORE_ACTIONS = "Другие действия";

function ActionMenuItem({ action }: { action: Action }) {
  const variant = action.destructive ? "destructive" : "default";
  if (action.disabledReason !== undefined) {
    // Пункт недоступен для выбора, поэтому причина стоит в самом пункте и читается как часть его имени.
    return (
      <MenuItem disabled variant={variant}>
        <span className="flex flex-col">
          <ActionContent action={action} />
          <span className="text-caption">{action.disabledReason}</span>
        </span>
      </MenuItem>
    );
  }
  if ("href" in action.run) {
    return (
      <MenuItem variant={variant} render={<Link href={action.run.href} />}>
        <ActionContent action={action} />
      </MenuItem>
    );
  }
  return (
    <MenuItem variant={variant} onClick={action.run.onSelect}>
      <ActionContent action={action} />
    </MenuItem>
  );
}

/**
 * Меню «…» с действиями. Опасные пункты идут последними и отделены разделителем автоматически;
 * пустой список — ничего не рендерится.
 */
export function ActionMenu({ actions }: { actions: readonly Action[] }) {
  if (actions.length === 0) {
    return null;
  }
  const regular = actions.filter((action) => !action.destructive);
  const destructive = actions.filter((action) => action.destructive);
  return (
    <Menu>
      <MenuTrigger render={<Button variant="ghost" size="icon" aria-label={MORE_ACTIONS} />}>
        <MoreHorizontalIcon aria-hidden="true" />
      </MenuTrigger>
      <MenuContent align="end">
        {regular.map((action) => (
          <ActionMenuItem key={action.label} action={action} />
        ))}
        {regular.length > 0 && destructive.length > 0 ? <MenuSeparator /> : null}
        {destructive.map((action) => (
          <ActionMenuItem key={action.label} action={action} />
        ))}
      </MenuContent>
    </Menu>
  );
}
