import { useLingui } from "@lingui/react/macro";
import { PanelLeftCloseIcon, PanelLeftOpenIcon } from "lucide-react";
import { Button } from "~/shared/ui/primitives";
import type { SidebarMode } from "../../lib/shell";

type Props = {
  mode: SidebarMode;
  /** Для `sheet` — открыт ли Sheet. */
  sheetOpen: boolean;
  onToggle: () => void;
};

/** Кнопка сайдбара в topbar. `aria-expanded` отражает, видно ли сейчас меню целиком. */
export function SidebarToggle({ mode, sheetOpen, onToggle }: Props) {
  const { t } = useLingui();
  const expanded = mode === "full" || (mode === "sheet" && sheetOpen);
  const Icon = mode === "full" ? PanelLeftCloseIcon : PanelLeftOpenIcon;
  return (
    <Button
      variant="ghost"
      size="icon"
      // Имя постоянное: состояние несёт aria-expanded, а не смена подписи (иначе скринридер читает его дважды).
      aria-label={t`Главное меню`}
      aria-expanded={expanded}
      onClick={onToggle}
    >
      <Icon aria-hidden="true" />
    </Button>
  );
}
