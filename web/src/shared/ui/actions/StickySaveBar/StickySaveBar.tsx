import { useLingui } from "@lingui/react/macro";
import { LoaderCircleIcon } from "lucide-react";
import type { WithClassName } from "~/shared/lib/core";
import { StickyBar } from "../../layout/StickyBar/StickyBar";
import { Button } from "../../primitives";
import { useSaveAnnouncements } from "./useSaveAnnouncements";

type SaveResult = { saved: number; total: number };

type SaveState = "idle" | "saving" | SaveResult;

type StickySaveBarProps = WithClassName<{
  /** Сколько полей изменено и ещё не сохранено; панель видна, пока это число больше нуля или идёт запись. */
  dirty: number;
  /**
   * `saving` — идёт запись; объект — итог. Частичный итог (сохранено меньше, чем было) виден на панели, пока
   * остаются несохранённые поля; полный только объявляется, панель после него уходит. Устаревший итог прошлой
   * сессии вызывающий сбрасывает в `idle` при следующей правке.
   */
  state: SaveState;
  onSave: () => void;
  onDiscard: () => void;
  /** Куда перевести фокус, если панель уходит, пока он на её кнопке (обычно заголовок карточки или форма). */
  returnFocus: () => HTMLElement | null;
}>;

type SaveView = { dirty: number; state: SaveState };

function resultOf(state: SaveState): SaveResult | null {
  return typeof state === "object" ? state : null;
}

function useSaveTexts({ dirty, state }: SaveView): { changed: string; summary: string | null } {
  const { t } = useLingui();
  const result = resultOf(state);
  const saved = result?.saved ?? 0;
  const total = result?.total ?? 0;
  return {
    changed: t`Изменено: ${dirty}`,
    summary: result === null ? null : t`Сохранено ${saved} из ${total}`,
  };
}

function SaveBarContent({
  view,
  onSave,
  onDiscard,
}: Pick<StickySaveBarProps, "onSave" | "onDiscard"> & { view: SaveView }) {
  const { t } = useLingui();
  const { changed, summary } = useSaveTexts(view);
  const saving = view.state === "saving";
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
      <p className="m-0 mr-auto flex flex-wrap items-center gap-x-1 tabular-nums">
        <span className="font-semibold">{changed}</span>
        {summary === null ? null : (
          <>
            <span>{"·"}</span>
            <span>{summary}</span>
          </>
        )}
      </p>
      <div className="flex items-center gap-2">
        <Button variant="ghost" disabled={saving} focusableWhenDisabled onClick={onDiscard}>
          {t`Отменить`}
        </Button>
        <Button disabled={saving} focusableWhenDisabled aria-busy={saving} onClick={onSave}>
          {saving ? (
            <LoaderCircleIcon aria-hidden="true" className="motion-safe:animate-spin" />
          ) : null}
          {t`Сохранить`}
        </Button>
      </div>
    </div>
  );
}

/**
 * Панель сохранения форм настроек: «Изменено: N», «Отменить», «Сохранить» и частичный итог «Сохранено K из N».
 * Объявления (первая правка сессии, итог) живут здесь, а не в панели: они переживают её уход.
 */
export function StickySaveBar({
  dirty,
  state,
  onSave,
  onDiscard,
  returnFocus,
  className,
}: StickySaveBarProps) {
  const { changed, summary } = useSaveTexts({ dirty, state });
  useSaveAnnouncements(dirty, changed, summary);
  const live = dirty > 0 || state === "saving";
  return (
    <StickyBar
      value={live ? { dirty, state } : null}
      returnFocus={returnFocus}
      {...(className === undefined ? {} : { className })}
    >
      {(view) => <SaveBarContent view={view} onSave={onSave} onDiscard={onDiscard} />}
    </StickyBar>
  );
}
