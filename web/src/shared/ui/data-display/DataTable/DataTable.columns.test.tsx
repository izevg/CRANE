import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { DataTable } from "~/shared/ui/data-display";
import { columns, getRowId, MANY_ROWS, ROW_141, rows, VERSION_HEAD } from "~test/dataTable";

describe("DataTable: колонки, окно и сортировка", () => {
  it("рисует заголовок колонки с приоритетом 1", () => {
    // Arrange
    // Act
    render(<DataTable columns={columns} data={rows} getRowId={getRowId} caption="Версии" />);

    // Assert
    expect(screen.getByRole("columnheader", { name: "Версия" })).toBeInTheDocument();
  });

  it("строки несут aria-rowindex, а aria-rowcount считает и строку шапки", () => {
    // Arrange
    // Act
    render(<DataTable columns={columns} data={rows} getRowId={getRowId} caption="Версии" />);

    // Assert
    expect(screen.getByRole("table")).toHaveAttribute("aria-rowcount", String(rows.length + 1));
    expect(screen.getByRole("row", { name: ROW_141 })).toHaveAttribute("aria-rowindex", "3");
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
});
