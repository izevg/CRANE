// i18n: не мигрирован
import { Autocomplete } from "@base-ui/react/autocomplete";
import { useLingui } from "@lingui/react";
import { useCallback, useMemo, useState } from "react";
import type { Action } from "~/shared/ui/actions";
import { Dialog, DialogContent, DialogTitle, Skeleton } from "~/shared/ui/primitives";
import { Kbd } from "~/shared/ui/status";
import {
  buildGroups,
  type NavigationEntry,
  optionText,
  type PackageHit,
  type PaletteGroup,
  type PaletteOption,
  pickTarget,
  resultsStatus,
} from "../../lib/palette/model";
import { PaletteRow } from "./PaletteRow";

const TITLE = "Командная палитра";
const INPUT_LABEL = "Перейти, найти пакет или действие";
const PLACEHOLDER = "Перейти, найти пакет или действие…";
const EMPTY = "Ничего не найдено";
const SEARCHING = "Ищем пакеты…";
const INPUT_CHANGE = "input-change";

const HINTS = [
  { keys: "↑↓", label: "выбор" },
  { keys: "Enter", label: "открыть" },
  { keys: "Esc", label: "закрыть" },
] as const;

const SKELETON_ROWS = ["first", "second", "third"] as const;

type Props = {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** Значение поля. Без него виджет хранит ввод сам и сбрасывает при закрытии. */
  query?: string;
  /**
   * Вызывается на каждое изменение ввода, без задержки: поле должно отвечать сразу.
   * Дебаунс запроса к серверу (рекомендуется 150 мс) делает вызывающий: он же владеет кэшем и отменой запросов.
   */
  onQueryChange: (query: string) => void;
  /** Разделы приложения; фильтруются локально по вводу. */
  navigation: readonly NavigationEntry[];
  /** Результат серверного поиска; локально не фильтруется. */
  packages: readonly PackageHit[];
  packagesLoading?: boolean;
  actions: readonly Action[];
  /** Роутера в виджете нет: переход выполняет приложение. */
  onNavigate: (href: string) => void;
};

function Skeletons() {
  return (
    <div aria-hidden="true" className="flex flex-col gap-2 px-2 py-2">
      {SKELETON_ROWS.map((row) => (
        <Skeleton key={row} className="h-5 w-full" />
      ))}
    </div>
  );
}

function Hints() {
  return (
    <div className="flex items-center gap-4 border-t border-border px-3 py-2 text-caption text-muted-foreground">
      {HINTS.map((hint) => (
        <span key={hint.keys} className="flex items-center gap-1">
          <Kbd>{hint.keys}</Kbd>
          {hint.label}
        </span>
      ))}
    </div>
  );
}

function Groups({ onPick }: { onPick: (option: PaletteOption) => void }) {
  return (
    <Autocomplete.List className="max-h-[min(22.5rem,60vh)] overflow-y-auto overscroll-contain p-1 outline-0 data-empty:p-0">
      {(group: PaletteGroup) => (
        <Autocomplete.Group key={group.value} items={group.items} className="block pb-1 last:pb-0">
          <Autocomplete.GroupLabel className="px-2 py-1.5 text-caption font-semibold text-muted-foreground">
            {group.value}
          </Autocomplete.GroupLabel>
          <Autocomplete.Collection>
            {(option: PaletteOption) => (
              <PaletteRow key={option.key} option={option} onPick={onPick} />
            )}
          </Autocomplete.Collection>
        </Autocomplete.Group>
      )}
    </Autocomplete.List>
  );
}

/**
 * Командная палитра (K1). Паттерн APG combobox + listbox даёт Base UI `Autocomplete`: список встроен в диалог
 * (`inline`), вручную роли и `aria-activedescendant` не расставляются.
 */
export function CommandPaletteView({
  open,
  onOpenChange,
  query,
  onQueryChange,
  navigation,
  packages,
  packagesLoading = false,
  actions,
  onNavigate,
}: Props) {
  const { _ } = useLingui();
  const [ownQuery, setOwnQuery] = useState("");
  const value = query ?? ownQuery;
  const groups = useMemo(
    () => buildGroups({ query: value, navigation, packages, actions }),
    [value, navigation, packages, actions],
  );

  const changeQuery = useCallback(
    (next: string) => {
      setOwnQuery(next);
      onQueryChange(next);
    },
    [onQueryChange],
  );

  const handleValueChange = useCallback(
    (next: string, details: { reason: string }) => {
      // Выбор пункта Base UI тоже пишет его текст в поле; это не ввод пользователя и не повод идти на сервер.
      if (details.reason === INPUT_CHANGE) {
        changeQuery(next);
      }
    },
    [changeQuery],
  );

  const handleOpenChange = useCallback(
    (next: boolean) => {
      if (!next && query === undefined) {
        setOwnQuery("");
      }
      onOpenChange(next);
    },
    [onOpenChange, query],
  );

  const handlePick = useCallback(
    (option: PaletteOption) => {
      const target = pickTarget(option);
      if ("href" in target) {
        onNavigate(target.href);
      } else {
        target.run();
      }
      handleOpenChange(false);
    },
    [onNavigate, handleOpenChange],
  );

  const results = resultsStatus({ query: value, groups });
  const statusText = packagesLoading ? SEARCHING : results === null ? null : _(results);
  const showSkeletons = packagesLoading && packages.length === 0;
  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent showCloseButton={false} size="palette" flush className="overflow-hidden">
        <DialogTitle className="sr-only">{TITLE}</DialogTitle>
        <Autocomplete.Root
          inline
          open
          autoHighlight
          items={groups}
          filter={null}
          value={value}
          onValueChange={handleValueChange}
          itemToStringValue={optionText}
        >
          <Autocomplete.Input
            aria-label={INPUT_LABEL}
            placeholder={PLACEHOLDER}
            className="h-12 w-full border-b border-border bg-transparent px-4 text-body text-foreground placeholder:text-hint focus-visible:-outline-offset-2"
          />
          <Autocomplete.Status className="sr-only">{statusText}</Autocomplete.Status>
          <Groups onPick={handlePick} />
          {showSkeletons ? <Skeletons /> : null}
          {packagesLoading ? null : (
            <Autocomplete.Empty className="px-4 py-6 text-center text-body text-muted-foreground empty:hidden">
              {EMPTY}
            </Autocomplete.Empty>
          )}
        </Autocomplete.Root>
        <Hints />
      </DialogContent>
    </Dialog>
  );
}
