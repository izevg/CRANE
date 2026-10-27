import { type RowData, type SortingState, useTable } from "@tanstack/react-table";
import { type ReactNode, useCallback, useMemo, useRef } from "react";
import { cx } from "~/shared/lib/core";
import { type Action, ActionButton } from "../../actions";
import {
  CHECKBOX_COLUMN,
  columnClass,
  type DataTableColumn,
  features,
  nextSelection,
} from "./columns";
import {
  DataRow,
  HeadRow,
  type HeadSelection,
  MessageRow,
  type RowCheckbox,
  SkeletonRows,
  Spacer,
} from "./DataTableRows";
import { useLoadMoreFocus, useRowFocus, useRowWindow, useScrollerLayout } from "./useRowFocus";

const SCROLL_MAX_HEIGHT = "max-h-[640px]";

type SelectionProps<TData> = {
  selected: ReadonlySet<string>;
  onChange: (next: ReadonlySet<string>) => void;
  /** Доступное имя чекбокса строки: «Выбрать версию 1.4.2». */
  label: (row: TData) => string;
};

type DataTableProps<TData extends RowData> = {
  columns: DataTableColumn<TData>[];
  data: readonly TData[];
  getRowId: (row: TData) => string;
  /** Доступное имя таблицы. */
  caption: string;
  loading?: boolean;
  /** Пустое состояние: рендерится вместо строк. */
  empty?: ReactNode;
  /** Ошибка загрузки: заменяет строки. */
  error?: ReactNode;
  /** Открыта панель деталей: колонки приоритета 3 скрываются. */
  compact?: boolean;
  selectedId?: string | null;
  /** Строка выбрана кликом, Enter или стрелками ↑/↓; без обработчика строки не фокусируются. */
  onRowSelect?: (id: string) => void;
  /** Выбор строк чекбоксами для массовых действий. */
  selection?: SelectionProps<TData>;
  /** Действия строки: закреплённая справа колонка с меню. */
  actions?: (row: TData) => readonly Action[];
  /** Курсорная догрузка: кнопка есть, только если передано действие. Счётчика «N из M» нет. */
  loadMore?: Action;
  /** Сортировка выполняется на сервере: таблица только сообщает о выборе. */
  sorting?: SortingState;
  onSortingChange?: (sorting: SortingState) => void;
  className?: string;
};

const NO_SORTING: SortingState = [];

function bodyState(hasError: boolean, loading: boolean, count: number) {
  if (hasError) {
    return "error";
  }
  if (loading) {
    return "loading";
  }
  return count === 0 ? "empty" : "rows";
}

type SkeletonCell = { key: string; className: string };

function skeletonCellsOf(
  columnCells: readonly SkeletonCell[],
  hasCheckbox: boolean,
  hasActions: boolean,
): SkeletonCell[] {
  return [
    ...(hasCheckbox ? [{ key: "checkbox", className: CHECKBOX_COLUMN }] : []),
    ...columnCells,
    ...(hasActions ? [{ key: "actions", className: "" }] : []),
  ];
}

/** Состояние чекбоксов: «выбрать все загруженные» в шапке и чекбокс каждой строки. */
function useSelectionModel<TData>(
  selection: SelectionProps<TData> | undefined,
  ids: readonly string[],
) {
  const toggleAll = useCallback(
    (on: boolean) => selection?.onChange(nextSelection(selection.selected, ids, on)),
    [selection, ids],
  );
  if (!selection) {
    return { head: null, checkboxOf: () => null };
  }
  const loaded = ids.filter((id) => selection.selected.has(id)).length;
  const head: HeadSelection = {
    allSelected: ids.length > 0 && loaded === ids.length,
    someSelected: loaded > 0,
    onToggleAll: toggleAll,
  };
  const checkboxOf = (id: string, row: TData): RowCheckbox => ({
    checked: selection.selected.has(id),
    label: selection.label(row),
    onChange: (on) => selection.onChange(nextSelection(selection.selected, [id], on)),
  });
  return { head, checkboxOf };
}

/**
 * Таблица данных: курсорная выдача, приоритеты колонок по ширине контейнера страницы, выбор строки для панели деталей,
 * выбор чекбоксами, виртуализация длинных списков. Колонки с приоритетом 2 и 3 прячутся CSS container queries
 * от `@container/page`; без такого контейнера видны все.
 */
export function DataTable<TData extends RowData>({
  columns,
  data,
  getRowId,
  caption,
  loading = false,
  empty,
  error,
  compact = false,
  selectedId = null,
  onRowSelect,
  selection,
  actions,
  loadMore,
  sorting = NO_SORTING,
  onSortingChange,
  className,
}: DataTableProps<TData>) {
  const scrollRef = useRef<HTMLDivElement>(null);

  const table = useTable<typeof features, TData>({
    features,
    columns,
    data: data as TData[],
    getRowId,
    manualSorting: true,
    enableMultiSort: false,
    state: { sorting },
    onSortingChange: (updater) => {
      onSortingChange?.(typeof updater === "function" ? updater(sorting) : updater);
    },
  });

  const rows = table.getRowModel().rows;
  const ids = useMemo(() => rows.map((row) => row.id), [rows]);
  useScrollerLayout(scrollRef, onRowSelect !== undefined);
  const view = useRowWindow(rows.length, scrollRef);
  const { activeId, setActiveId, focusAt } = useRowFocus({
    scrollRef,
    ids,
    scrollToIndex: view.virtual ? view.scrollToIndex : undefined,
  });
  const markLoad = useLoadMoreFocus({
    rowCount: rows.length,
    hasLoadMore: loadMore !== undefined,
    focusAt,
    inner: onRowSelect === undefined,
  });

  const move = useCallback(
    (index: number) => {
      const target = ids[index];
      if (target !== undefined) {
        onRowSelect?.(target);
        focusAt(index);
      }
    },
    [ids, onRowSelect, focusAt],
  );
  const { head: headSelection, checkboxOf } = useSelectionModel(selection, ids);

  const leaf = table.getAllLeafColumns();
  const span = Math.max(
    leaf.length + Number(selection !== undefined) + Number(actions !== undefined),
    1,
  );
  const rendered = new Set(view.items.map((item) => rows[item.index]?.id));
  const preferred = [selectedId, activeId].find((id) => id !== null && rendered.has(id));
  const tabbableId = preferred ?? rows[view.items[0]?.index ?? 0]?.id;
  const skeletonCells = skeletonCellsOf(
    leaf.map((column) => ({
      key: column.id,
      className: columnClass(column.columnDef.meta, compact),
    })),
    selection !== undefined,
    actions !== undefined,
  );
  const state = bodyState(Boolean(error), loading, rows.length);

  return (
    <div className={className}>
      <div
        ref={scrollRef}
        className={cx(
          "relative overflow-auto scroll-pt-10 scroll-ps-[var(--pin-start,0px)] scroll-pe-[var(--pin-end,0px)]",
          view.virtual && SCROLL_MAX_HEIGHT,
        )}
      >
        <table
          aria-label={caption}
          aria-rowcount={rows.length + 1}
          aria-busy={loading || undefined}
          className="w-full text-body"
        >
          <thead className="sticky top-0 z-10 bg-card">
            {table.getHeaderGroups().map((group) => (
              <HeadRow
                key={group.id}
                headers={group.headers}
                compact={compact}
                sortable={onSortingChange !== undefined}
                selection={headSelection}
                hasActions={actions !== undefined}
              />
            ))}
          </thead>
          <tbody>
            {state === "loading" ? <SkeletonRows cells={skeletonCells} /> : null}
            {state === "error" ? <MessageRow span={span}>{error}</MessageRow> : null}
            {state === "empty" ? <MessageRow span={span}>{empty}</MessageRow> : null}
            {state === "rows" ? <Spacer height={view.padTop} span={span} /> : null}
            {state === "rows"
              ? view.items.map((item) => {
                  const row = rows[item.index];
                  return row ? (
                    <DataRow
                      key={row.id}
                      row={row}
                      index={item.index}
                      selected={selectedId === row.id}
                      focusable={onRowSelect !== undefined}
                      tabbable={row.id === tabbableId}
                      compact={compact}
                      checkbox={checkboxOf(row.id, row.original)}
                      actions={actions ? actions(row.original) : null}
                      onSelect={onRowSelect}
                      onFocusRow={setActiveId}
                      onMove={move}
                    />
                  ) : null;
                })
              : null}
            {state === "rows" ? <Spacer height={view.padBottom} span={span} /> : null}
          </tbody>
        </table>
      </div>
      {loadMore ? (
        <div className="flex justify-center border-t p-3" onClickCapture={markLoad}>
          <ActionButton action={loadMore} variant="secondary" />
        </div>
      ) : null}
    </div>
  );
}
