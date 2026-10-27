import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { DataTable } from "~/shared/ui/data-display";
import { columns, focusRow, getRowId, ROW_140, ROW_141, rows } from "~test/dataTable";

describe("DataTable: выбор строк и клавиатура", () => {
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

  it("без onRowSelect строки не фокусируются", () => {
    // Arrange
    // Act
    render(<DataTable columns={columns} data={rows} getRowId={getRowId} caption="Версии" />);

    // Assert
    expect(screen.getByRole("row", { name: ROW_140 })).not.toHaveAttribute("tabindex");
  });
});
