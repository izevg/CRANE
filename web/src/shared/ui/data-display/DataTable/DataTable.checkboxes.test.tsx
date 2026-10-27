import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { DataTable } from "~/shared/ui/data-display";
import { columns, getRowId, rows, type Version } from "~test/dataTable";

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
