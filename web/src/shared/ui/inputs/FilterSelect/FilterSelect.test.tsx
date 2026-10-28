import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { FilterSelect } from "~/shared/ui/inputs";

const OPTIONS = [
  { value: "npm", label: "npm" },
  { value: "maven", label: "Maven" },
];

describe("FilterSelect", () => {
  it("значение null показывает подпись «все»", () => {
    // Arrange
    const onValueChange = vi.fn();

    // Act
    render(
      <FilterSelect
        label="Формат"
        allLabel="Все форматы"
        value={null}
        options={OPTIONS}
        onValueChange={onValueChange}
      />,
    );

    // Assert
    expect(screen.getByRole("combobox", { name: "Формат" })).toHaveTextContent("Все форматы");
  });

  it("выбор опции вызывает onValueChange с её значением", async () => {
    // Arrange
    const user = userEvent.setup();
    const onValueChange = vi.fn();
    render(
      <FilterSelect
        label="Формат"
        allLabel="Все форматы"
        value={null}
        options={OPTIONS}
        onValueChange={onValueChange}
      />,
    );
    await user.click(screen.getByRole("combobox", { name: "Формат" }));

    // Act
    await user.click(await screen.findByRole("option", { name: "Maven" }));

    // Assert
    expect(onValueChange).toHaveBeenCalledWith("maven");
  });

  it("выбор «все» вызывает onValueChange с null", async () => {
    // Arrange
    const user = userEvent.setup();
    const onValueChange = vi.fn();
    render(
      <FilterSelect
        label="Формат"
        allLabel="Все форматы"
        value="npm"
        options={OPTIONS}
        onValueChange={onValueChange}
      />,
    );
    await user.click(screen.getByRole("combobox", { name: "Формат" }));

    // Act
    await user.click(await screen.findByRole("option", { name: "Все форматы" }));

    // Assert
    expect(onValueChange).toHaveBeenCalledWith(null);
  });

  it("значение вне списка показывается ключом, а не сбрасывается в «все»", () => {
    // Arrange
    const onValueChange = vi.fn();

    // Act
    render(
      <FilterSelect
        label="Формат"
        allLabel="Все форматы"
        value="pypi"
        options={OPTIONS}
        onValueChange={onValueChange}
      />,
    );

    // Assert
    expect(screen.getByRole("combobox", { name: "Формат" })).toHaveTextContent("pypi");
  });
});
