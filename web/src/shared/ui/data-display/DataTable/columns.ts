import {
  type CellData,
  type ColumnDef,
  createColumnHelper,
  type RowData,
  rowSortingFeature,
  type TableFeatures,
  tableFeatures,
} from "@tanstack/react-table";

/** Сортировку выполняет сервер (курсорная выдача), поэтому в таблице только признак и состояние сортировки. */
export const features = tableFeatures({ rowSortingFeature });

export type DataTableColumn<TData extends RowData> = ColumnDef<typeof features, TData>;

/** Типизированный помощник колонок: `columnHelper<Row>().accessor(...)`. */
export function columnHelper<TData extends RowData>() {
  return createColumnHelper<typeof features, TData>();
}

/** 1 — всегда, 2 — от `lg`, 3 — от `2xl` и без открытой панели деталей. */
type ColumnPriority = 1 | 2 | 3;

declare module "@tanstack/react-table" {
  interface ColumnMeta<
    in out TFeatures extends TableFeatures,
    in out TData extends RowData,
    TValue extends CellData = CellData,
  > {
    /** Приоритет колонки; без значения колонка считается обязательной (1). */
    priority?: ColumnPriority;
    /** Числовая колонка: выравнивание вправо, табличные цифры. */
    numeric?: boolean;
  }
}

const LOWEST_PRIORITY = 3;

// Классы записаны целиком: Tailwind находит их по тексту исходников.
// Пороги считаются по ширине контейнера страницы, а не окна: окно минус сайдбар (248 px) и поля.
// P2 с окна lg (1024): 1024 − 248 − 2×24 ≈ 728 px → 45rem (720 px).
// P3 с окна 2xl (1536): 1536 − 248 − 2×32 ≈ 1224 px → 76rem (1216 px).
// Видимость считается от контейнера `@container/page`, который объявляет шелл. Без такого контейнера
// условие `max-width` не срабатывает, и все колонки остаются видимыми.
const PRIORITY_CLASS = {
  1: "",
  2: "@max-[45rem]/page:hidden",
  3: "@max-[76rem]/page:hidden",
} satisfies Record<ColumnPriority, string>;

type PriorityMeta = { priority?: ColumnPriority; numeric?: boolean } | undefined;

/** Классы ячейки и заголовка колонки: видимость по приоритету и выравнивание числовых колонок. */
export function columnClass(meta: PriorityMeta, compact: boolean): string {
  const priority = meta?.priority ?? 1;
  const visibility = compact && priority === LOWEST_PRIORITY ? "hidden" : PRIORITY_CLASS[priority];
  return [visibility, meta?.numeric ? "text-right tabular-nums" : ""].filter(Boolean).join(" ");
}

/** Ширина колонки с чекбоксом (`w-10`); от неё отсчитывается закрепление первой колонки данных. */
export const CHECKBOX_COLUMN = "w-10 min-w-10";

// Ячейки закреплённых колонок берут фон строки (`bg-inherit`): он задан явно и меняется при hover и выборе.
export const PIN_LEFT = "sticky left-0 z-[1] bg-inherit";
export const PIN_LEFT_AFTER_CHECKBOX = "sticky left-10 z-[1] bg-inherit";
export const PIN_RIGHT = "sticky right-0 z-[1] bg-inherit";
export const HEAD_PIN_LEFT = "sticky left-0 z-[2] bg-card";
export const HEAD_PIN_LEFT_AFTER_CHECKBOX = "sticky left-10 z-[2] bg-card";
export const HEAD_PIN_RIGHT = "sticky right-0 z-[2] bg-card";
export const SELECTED_BAR =
  "group-aria-selected/row:shadow-[inset_2px_0_0_var(--row-selected-bar)]";

export const CELL = "h-11 px-3 py-0 text-left align-middle whitespace-nowrap";
export const HEAD =
  "h-10 px-3 text-left align-middle text-overline whitespace-nowrap text-muted-foreground";

/** Набор выбранных id после включения или снятия группы строк. */
export function nextSelection(
  selected: ReadonlySet<string>,
  ids: readonly string[],
  on: boolean,
): ReadonlySet<string> {
  const next = new Set(selected);
  for (const id of ids) {
    if (on) {
      next.add(id);
    } else {
      next.delete(id);
    }
  }
  return next;
}
