import { act, screen } from "@testing-library/react";
import { columnHelper, type DataTableColumn } from "~/shared/ui/data-display";

export type Version = { id: string; name: string; downloads: number; team: string };

const helper = columnHelper<Version>();
export const columns: DataTableColumn<Version>[] = helper.columns([
  helper.accessor("name", { header: "Версия", meta: { priority: 1 } }),
  helper.accessor("downloads", { header: "Загрузки", meta: { priority: 2, numeric: true } }),
  helper.accessor("team", { header: "Команда", meta: { priority: 3 } }),
]);
export const rows: Version[] = [
  { id: "1", name: "1.4.0", downloads: 12, team: "web" },
  { id: "2", name: "1.4.1", downloads: 40, team: "web" },
  { id: "3", name: "1.4.2", downloads: 7, team: "core" },
];
export const MANY_ROWS = 300;
export const SKELETON_ROWS_WITH_HEADER = 9;
export const EMPTY_TEXT = "Версий пока нет";
export const ERROR_TEXT = "Не удалось загрузить";
export const ROW_140 = /1\.4\.0/;
export const ROW_141 = /1\.4\.1/;
export const ROW_150 = /1\.5\.0/;
export const VERSION_HEAD = /Версия/;
export const getRowId = (row: Version) => row.id;

export function focusRow(name: RegExp) {
  const row = screen.getByRole("row", { name });
  act(() => row.focus());
  return row;
}

export const MORE: Version = { id: "4", name: "1.5.0", downloads: 1, team: "web" };
