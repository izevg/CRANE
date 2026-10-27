import { PanelLeftCloseIcon, PanelLeftOpenIcon } from "lucide-react";
import { Button } from "~/shared/ui/primitives";

// Имя постоянное: состояние несёт aria-expanded, а не смена подписи (иначе скринридер читает его дважды).
const LABEL = "Главное меню";

/** `sheet` — меню в Sheet, `full` — развёрнутая колонка 248 px, `rail` — рельса 64 px. */
export type SidebarMode = "full" | "rail" | "sheet";

type Props = {
  mode: SidebarMode;
  /** Для `sheet` — открыт ли Sheet. */
  sheetOpen: boolean;
  onToggle: () => void;
};

/** Кнопка сайдбара в topbar. `aria-expanded` отражает, видно ли сейчас меню целиком. */
export function SidebarToggle({ mode, sheetOpen, onToggle }: Props) {
  const expanded = mode === "full" || (mode === "sheet" && sheetOpen);
  const Icon = mode === "full" ? PanelLeftCloseIcon : PanelLeftOpenIcon;
  return (
    <Button
      variant="ghost"
      size="icon"
      aria-label={LABEL}
      aria-expanded={expanded}
      onClick={onToggle}
    >
      <Icon aria-hidden="true" />
    </Button>
  );
}
