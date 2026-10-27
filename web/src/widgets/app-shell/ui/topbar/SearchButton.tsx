import { SearchIcon } from "lucide-react";
import { Kbd } from "~/shared/ui/status";

const LABEL = "Поиск и переходы Ctrl K";
const VISIBLE = "Поиск и переходы";
const SHORTCUT = "Ctrl K";

/** Открывает палитру команд (сама палитра приходит из другого виджета). Видимый текст входит в доступное имя. */
export function SearchButton({ onOpen }: { onOpen: () => void }) {
  return (
    <button
      type="button"
      aria-label={LABEL}
      aria-keyshortcuts="Control+K Meta+K"
      aria-haspopup="dialog"
      onClick={onOpen}
      className="flex h-9 min-w-9 items-center justify-center gap-2 rounded-md border border-input bg-card px-2 text-left transition-colors duration-(--dur-fast) hover:bg-accent sm:min-w-52 sm:max-w-md sm:flex-1 sm:justify-start sm:px-3"
    >
      <SearchIcon aria-hidden="true" className="size-4 shrink-0 text-muted-foreground" />
      <span className="hidden min-w-0 flex-1 truncate text-body text-muted-foreground sm:inline">
        {VISIBLE}
      </span>{" "}
      <Kbd aria-hidden="true" className="hidden sm:inline-flex">
        {SHORTCUT}
      </Kbd>
    </button>
  );
}
