import { useLingui } from "@lingui/react";
import { FlexRender, type Header, type Row, type RowData } from "@tanstack/react-table";
import { ArrowDownIcon, ArrowUpDownIcon, ArrowUpIcon } from "lucide-react";
import type { KeyboardEvent, MouseEvent, ReactNode } from "react";
import { cx } from "~/shared/lib/core";
import { type Action, ActionMenu } from "../../actions";
import { Checkbox, Skeleton } from "../../primitives";
import { TABLE_ACTIONS, TABLE_LOADING, TABLE_SELECT_ALL } from "../text/messages";
import {
  CELL,
  CHECKBOX_COLUMN,
  columnClass,
  edgeClass,
  type features,
  HEAD,
  HEAD_PIN_LEFT,
  HEAD_PIN_LEFT_AFTER_CHECKBOX,
  HEAD_PIN_RIGHT,
  PIN_LEFT,
  PIN_LEFT_AFTER_CHECKBOX,
  PIN_RIGHT,
} from "./columns";

const SKELETON_ROWS = 8;
const SKELETON_KEYS = Array.from({ length: SKELETON_ROWS }, (_, index) => `skeleton-${index}`);

const INTERACTIVE = "a,button,input,select,textarea,[role=button],[role=link],[role=checkbox]";
const ARIA_SORT = { asc: "ascending", desc: "descending" } as const;

function SortIcon({ direction }: { direction: false | "asc" | "desc" }) {
  if (direction === "asc") {
    return <ArrowUpIcon aria-hidden="true" className="size-4" />;
  }
  if (direction === "desc") {
    return <ArrowDownIcon aria-hidden="true" className="size-4" />;
  }
  return <ArrowUpDownIcon aria-hidden="true" className="size-4 opacity-50" />;
}

type HeadCellProps<TData extends RowData> = {
  header: Header<typeof features, TData>;
  pin: string | false;
  compact: boolean;
  sortable: boolean;
};

function HeadCell<TData extends RowData>({ header, pin, compact, sortable }: HeadCellProps<TData>) {
  const { column } = header;
  const meta = column.columnDef.meta;
  const direction = column.getIsSorted();
  const canSort = sortable && column.getCanSort();
  return (
    <th
      scope="col"
      data-pin={pin ? "start" : undefined}
      aria-sort={canSort ? (direction === false ? "none" : ARIA_SORT[direction]) : undefined}
      className={cx(HEAD, columnClass(meta, compact), pin)}
    >
      {canSort ? (
        <button
          type="button"
          onClick={column.getToggleSortingHandler()}
          className={cx(
            "flex w-fit items-center gap-1 rounded-sm uppercase hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring",
            meta?.numeric && "ml-auto flex-row-reverse",
          )}
        >
          <FlexRender header={header} />
          <SortIcon direction={direction} />
        </button>
      ) : (
        <FlexRender header={header} />
      )}
    </th>
  );
}

export type HeadSelection = {
  allSelected: boolean;
  someSelected: boolean;
  onToggleAll: (on: boolean) => void;
};

type HeadRowProps<TData extends RowData> = {
  headers: readonly Header<typeof features, TData>[];
  compact: boolean;
  sortable: boolean;
  selection: HeadSelection | null;
  hasActions: boolean;
};

/** Строка заголовков: чекбокс «выбрать все загруженные», колонки данных, служебная колонка действий. */
export function HeadRow<TData extends RowData>({
  headers,
  compact,
  sortable,
  selection,
  hasActions,
}: HeadRowProps<TData>) {
  const { _ } = useLingui();
  const firstPin = selection ? HEAD_PIN_LEFT_AFTER_CHECKBOX : HEAD_PIN_LEFT;
  return (
    <tr aria-rowindex={1} className="h-10 border-b">
      {selection ? (
        <th scope="col" data-pin="start" className={cx(HEAD, CHECKBOX_COLUMN, HEAD_PIN_LEFT)}>
          <Checkbox
            checked={selection.allSelected}
            indeterminate={selection.someSelected && !selection.allSelected}
            onCheckedChange={selection.onToggleAll}
            aria-label={_(TABLE_SELECT_ALL)}
          />
        </th>
      ) : null}
      {headers.map((header, position) => (
        <HeadCell
          key={header.id}
          header={header}
          pin={position === 0 && firstPin}
          compact={compact}
          sortable={sortable}
        />
      ))}
      {hasActions ? (
        <th scope="col" data-pin="end" className={cx(HEAD, HEAD_PIN_RIGHT)}>
          <span className="sr-only">{_(TABLE_ACTIONS)}</span>
        </th>
      ) : null}
    </tr>
  );
}

export type RowCheckbox = { checked: boolean; label: string; onChange: (next: boolean) => void };

type DataRowProps<TData extends RowData> = {
  row: Row<typeof features, TData>;
  index: number;
  selected: boolean;
  focusable: boolean;
  tabbable: boolean;
  compact: boolean;
  checkbox: RowCheckbox | null;
  actions: readonly Action[] | null;
  onSelect: ((id: string) => void) | undefined;
  onFocusRow: (id: string) => void;
  onMove: (index: number) => void;
};

function tabIndexOf(focusable: boolean, tabbable: boolean): 0 | -1 | undefined {
  if (!focusable) {
    return undefined;
  }
  return tabbable ? 0 : -1;
}

/**
 * Строка данных. Фокусируема только при `onSelect`: ↑/↓ меняют выбранную строку, Enter и клик выбирают текущую.
 * Нажатия во вложенных чекбоксе, ссылке и меню строке не принадлежат.
 */
export function DataRow<TData extends RowData>({
  row,
  index,
  selected,
  focusable,
  tabbable,
  compact,
  checkbox,
  actions,
  onSelect,
  onFocusRow,
  onMove,
}: DataRowProps<TData>) {
  const handleClick = (event: MouseEvent<HTMLTableRowElement>) => {
    if (!(event.target as HTMLElement).closest(INTERACTIVE)) {
      onSelect?.(row.id);
    }
  };
  const handleKeyDown = (event: KeyboardEvent<HTMLTableRowElement>) => {
    if (event.target !== event.currentTarget) {
      return;
    }
    if (event.key === "Enter") {
      event.preventDefault();
      onSelect?.(row.id);
    } else if (event.key === "ArrowDown" || event.key === "ArrowUp") {
      event.preventDefault();
      onMove(index + (event.key === "ArrowDown" ? 1 : -1));
    }
  };
  const handleFocus = () => onFocusRow(row.id);
  const pinFirst = checkbox ? PIN_LEFT_AFTER_CHECKBOX : PIN_LEFT;
  const dataCells = row.getAllCells();
  const offset = checkbox ? 1 : 0;
  const total = offset + dataCells.length + (actions ? 1 : 0);
  return (
    <tr
      data-row-id={row.id}
      aria-rowindex={index + 2}
      aria-selected={focusable ? selected : undefined}
      tabIndex={tabIndexOf(focusable, tabbable)}
      onFocus={focusable ? handleFocus : undefined}
      onClick={focusable ? handleClick : undefined}
      onKeyDown={focusable ? handleKeyDown : undefined}
      className={cx(
        "group/row h-11 border-b bg-card hover:bg-(--row-hover-bg) hover:transition-[background-color] hover:duration-(--dur-instant) hover:ease-(--ease-out) aria-selected:bg-(--row-selected-bg) focus-visible:outline-hidden forced-colors:focus-visible:outline-2 forced-colors:focus-visible:-outline-offset-2 forced-colors:aria-selected:outline-2 forced-colors:aria-selected:-outline-offset-2 forced-colors:aria-selected:outline-[Highlight]",
        focusable && "cursor-pointer",
      )}
    >
      {checkbox ? (
        <td className={cx(CELL, CHECKBOX_COLUMN, PIN_LEFT, edgeClass(0, total))}>
          <Checkbox
            checked={checkbox.checked}
            onCheckedChange={checkbox.onChange}
            aria-label={checkbox.label}
          />
        </td>
      ) : null}
      {dataCells.map((cell, position) => (
        <td
          key={cell.id}
          className={cx(
            CELL,
            columnClass(cell.column.columnDef.meta, compact),
            position === 0 && pinFirst,
            edgeClass(offset + position, total),
          )}
        >
          <FlexRender cell={cell} />
        </td>
      ))}
      {actions ? (
        <td className={cx(CELL, PIN_RIGHT, edgeClass(total - 1, total))}>
          <ActionMenu actions={actions} />
        </td>
      ) : null}
    </tr>
  );
}

/** Скелетон повторяет сетку колонок, чтобы раскладка не прыгала при появлении данных. */
export function SkeletonRows({ cells }: { cells: readonly { key: string; className: string }[] }) {
  const { _ } = useLingui();
  return (
    <>
      {SKELETON_KEYS.map((rowKey, rowIndex) => (
        <tr key={rowKey} className="h-11 border-b">
          {cells.map((cell, cellIndex) => (
            <td key={cell.key} className={cx(CELL, cell.className)}>
              {rowIndex === 0 && cellIndex === 0 ? (
                <span className="sr-only">{_(TABLE_LOADING)}</span>
              ) : null}
              <Skeleton className="h-4 w-full max-w-40" />
            </td>
          ))}
        </tr>
      ))}
    </>
  );
}

export function MessageRow({ span, children }: { span: number; children: ReactNode }) {
  return (
    <tr>
      <td colSpan={span} className="p-0">
        {children}
      </td>
    </tr>
  );
}

/** Пустая строка высотой в пропущенные виртуализацией строки. */
export function Spacer({ height, span }: { height: number; span: number }) {
  if (height <= 0) {
    return null;
  }
  return (
    <tr>
      <td colSpan={span} className="p-0" style={{ height }} />
    </tr>
  );
}
