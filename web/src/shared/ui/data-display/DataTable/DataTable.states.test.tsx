import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Action } from "~/shared/ui/actions";
import { DataTable } from "~/shared/ui/data-display";
import {
  columns,
  EMPTY_TEXT,
  ERROR_TEXT,
  getRowId,
  MORE,
  ROW_150,
  rows,
  SKELETON_ROWS_WITH_HEADER,
  type Version,
} from "~test/dataTable";

describe("DataTable: загрузка, пусто, ошибка и догрузка", () => {
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
