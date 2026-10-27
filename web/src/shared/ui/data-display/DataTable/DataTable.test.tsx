import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Action } from "~/shared/ui/actions";
import { columnHelper, DataTable, type DataTableColumn } from "~/shared/ui/data-display";

type Version = { id: string; name: string; downloads: number; team: string };

const helper = columnHelper<Version>();
const columns: DataTableColumn<Version>[] = helper.columns([
  helper.accessor("name", { header: "Версия", meta: { priority: 1 } }),
  helper.accessor("downloads", { header: "Загрузки", meta: { priority: 2, numeric: true } }),
  helper.accessor("team", { header: "Команда", meta: { priority: 3 } }),
]);
const rows: Version[] = [
  { id: "1", name: "1.4.0", downloads: 12, team: "web" },
  { id: "2", name: "1.4.1", downloads: 40, team: "web" },
  { id: "3", name: "1.4.2", downloads: 7, team: "core" },
];
const MANY_ROWS = 300;
const SKELETON_ROWS_WITH_HEADER = 9;
const EMPTY_TEXT = "Версий пока нет";
const ERROR_TEXT = "Не удалось загрузить";
const ROW_140 = /1\.4\.0/;
const ROW_141 = /1\.4\.1/;
const ROW_150 = /1\.5\.0/;
const VERSION_HEAD = /Версия/;
const getRowId = (row: Version) => row.id;

function focusRow(name: RegExp) {
  const row = screen.getByRole("row", { name });
  act(() => row.focus());
  return row;
}

const MORE: Version = { id: "4", name: "1.5.0", downloads: 1, team: "web" };

describe("DataTable", () => {
  it("рисует заголовок колонки с приоритетом 1", () => {
    // Arrange
    // Act
    render(<DataTable columns={columns} data={rows} getRowId={getRowId} caption="Версии" />);

    // Assert
    expect(screen.getByRole("columnheader", { name: "Версия" })).toBeInTheDocument();
  });

  it("без пропа loadMore кнопки «Загрузить ещё» нет", () => {
    // Arrange
    // Act
    render(<DataTable columns={columns} data={rows} getRowId={getRowId} caption="Версии" />);

    // Assert
    expect(screen.queryByRole("button", { name: "Загрузить ещё" })).not.toBeInTheDocument();
  });

  it("с действием loadMore кнопка вызывает его обработчик", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const loadMore: Action = { label: "Загрузить ещё", run: { onSelect } };
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        loadMore={loadMore}
      />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Загрузить ещё" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("выбранная строка помечена aria-selected", () => {
    // Arrange
    // Act
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        selectedId="2"
        onRowSelect={vi.fn()}
      />,
    );

    // Assert
    expect(screen.getByRole("row", { selected: true })).toHaveTextContent("1.4.1");
  });

  it("клик по строке выбирает её", async () => {
    // Arrange
    const user = userEvent.setup();
    const onRowSelect = vi.fn();
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        onRowSelect={onRowSelect}
      />,
    );

    // Act
    await user.click(screen.getByText("1.4.2"));

    // Assert
    expect(onRowSelect).toHaveBeenCalledWith("3");
  });

  it("стрелка вниз выбирает следующую строку, фокус остаётся в таблице", async () => {
    // Arrange
    const user = userEvent.setup();
    const onRowSelect = vi.fn();
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        selectedId="1"
        onRowSelect={onRowSelect}
      />,
    );
    focusRow(ROW_140);

    // Act
    await user.keyboard("{ArrowDown}");

    // Assert
    expect(onRowSelect).toHaveBeenCalledWith("2");
    expect(screen.getByRole("row", { name: ROW_141 })).toHaveFocus();
  });

  it("стрелка вверх на первой строке ничего не выбирает", async () => {
    // Arrange
    const user = userEvent.setup();
    const onRowSelect = vi.fn();
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        selectedId="1"
        onRowSelect={onRowSelect}
      />,
    );
    focusRow(ROW_140);

    // Act
    await user.keyboard("{ArrowUp}");

    // Assert
    expect(onRowSelect).not.toHaveBeenCalled();
  });

  it("строки несут aria-rowindex, а aria-rowcount считает и строку шапки", () => {
    // Arrange
    // Act
    render(<DataTable columns={columns} data={rows} getRowId={getRowId} caption="Версии" />);

    // Assert
    expect(screen.getByRole("table")).toHaveAttribute("aria-rowcount", String(rows.length + 1));
    expect(screen.getByRole("row", { name: ROW_141 })).toHaveAttribute("aria-rowindex", "3");
  });

  it("без onRowSelect строки не фокусируются", () => {
    // Arrange
    // Act
    render(<DataTable columns={columns} data={rows} getRowId={getRowId} caption="Версии" />);

    // Assert
    expect(screen.getByRole("row", { name: ROW_140 })).not.toHaveAttribute("tabindex");
  });

  it("пустые данные показывают слот empty", () => {
    // Arrange
    // Act
    render(
      <DataTable
        columns={columns}
        data={[]}
        getRowId={getRowId}
        caption="Версии"
        empty={<p>{EMPTY_TEXT}</p>}
      />,
    );

    // Assert
    expect(screen.getByText(EMPTY_TEXT)).toBeInTheDocument();
  });

  it("ошибка заменяет строки слотом error", () => {
    // Arrange
    // Act
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        error={<p>{ERROR_TEXT}</p>}
      />,
    );

    // Assert
    expect(screen.getByText(ERROR_TEXT)).toBeInTheDocument();
    expect(screen.queryByText("1.4.0")).not.toBeInTheDocument();
  });

  it("загрузка рисует восемь строк-скелетонов вместо данных и не показывает empty", () => {
    // Arrange
    // Act
    render(
      <DataTable
        columns={columns}
        data={[]}
        getRowId={getRowId}
        caption="Версии"
        loading
        empty={<p>{EMPTY_TEXT}</p>}
      />,
    );

    // Assert
    expect(screen.getAllByRole("row")).toHaveLength(SKELETON_ROWS_WITH_HEADER);
    expect(screen.queryByText(EMPTY_TEXT)).not.toBeInTheDocument();
  });

  it("клик по сортируемому заголовку сообщает новую сортировку", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSortingChange = vi.fn();
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        onSortingChange={onSortingChange}
      />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: VERSION_HEAD }));

    // Assert
    expect(onSortingChange).toHaveBeenCalledWith([{ id: "name", desc: false }]);
  });

  it("колонка действий даёт меню у каждой строки", () => {
    // Arrange
    const actions = () => [{ label: "Заблокировать", run: { onSelect: vi.fn() } }];

    // Act
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        actions={actions}
      />,
    );

    // Assert
    expect(screen.getAllByRole("button", { name: "Другие действия" })).toHaveLength(rows.length);
  });

  it("длинный список рисует в DOM только окно строк", () => {
    // Arrange
    const many = Array.from({ length: MANY_ROWS }, (_, index) => ({
      id: String(index),
      name: `v${index}`,
      downloads: index,
      team: "web",
    }));

    // Act
    render(<DataTable columns={columns} data={many} getRowId={getRowId} caption="Версии" />);

    // Assert
    expect(screen.getAllByRole("row").length).toBeLessThan(MANY_ROWS);
  });

  it("после догрузки без следующей страницы фокус уходит на первую новую строку", async () => {
    // Arrange
    const user = userEvent.setup();
    const withRows = (data: Version[], loadMore?: Action) => (
      <DataTable
        columns={columns}
        data={data}
        getRowId={getRowId}
        caption="Версии"
        onRowSelect={vi.fn()}
        {...(loadMore ? { loadMore } : {})}
      />
    );
    const loadMore: Action = {
      label: "Загрузить ещё",
      run: { onSelect: () => view.rerender(withRows([...rows, MORE])) },
    };
    const view = render(withRows(rows, loadMore));

    // Act
    await user.click(screen.getByRole("button", { name: "Загрузить ещё" }));

    // Assert
    expect(screen.getByRole("row", { name: ROW_150 })).toHaveFocus();
  });
});

describe("DataTable: выбор чекбоксами", () => {
  const label = (row: Version) => `Выбрать ${row.name}`;

  it("Space на чекбоксе строки добавляет её в выбранные", async () => {
    // Arrange
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        selection={{ selected: new Set(), onChange, label }}
      />,
    );
    act(() => screen.getByRole("checkbox", { name: "Выбрать 1.4.1" }).focus());

    // Act
    await user.keyboard(" ");

    // Assert
    expect(onChange).toHaveBeenCalledWith(new Set(["2"]));
  });

  it("«выбрать все загруженные» выбирает все строки", async () => {
    // Arrange
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        selection={{ selected: new Set(), onChange, label }}
      />,
    );

    // Act
    await user.click(screen.getByRole("checkbox", { name: "Выбрать все загруженные" }));

    // Assert
    expect(onChange).toHaveBeenCalledWith(new Set(["1", "2", "3"]));
  });

  it("повторный клик по «выбрать все» снимает загруженные строки", async () => {
    // Arrange
    const user = userEvent.setup();
    const onChange = vi.fn();
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        selection={{ selected: new Set(["1", "2", "3"]), onChange, label }}
      />,
    );

    // Act
    await user.click(screen.getByRole("checkbox", { name: "Выбрать все загруженные" }));

    // Assert
    expect(onChange).toHaveBeenCalledWith(new Set());
  });

  it("клик по чекбоксу строки не выбирает саму строку", async () => {
    // Arrange
    const user = userEvent.setup();
    const onRowSelect = vi.fn();
    render(
      <DataTable
        columns={columns}
        data={rows}
        getRowId={getRowId}
        caption="Версии"
        onRowSelect={onRowSelect}
        selection={{ selected: new Set(), onChange: vi.fn(), label }}
      />,
    );

    // Act
    await user.click(screen.getByRole("checkbox", { name: "Выбрать 1.4.0" }));

    // Assert
    expect(onRowSelect).not.toHaveBeenCalled();
  });
});
