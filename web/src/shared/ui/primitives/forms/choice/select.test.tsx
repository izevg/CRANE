// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовые компоненты-помощники не экспортируются
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectLabel,
  SelectTrigger,
  SelectValue,
} from "~/shared/ui/primitives";

const ITEMS = [
  { value: "npm", label: "npm" },
  { value: "maven", label: "Maven" },
];

function FormatSelect() {
  return (
    <Select items={ITEMS}>
      <SelectTrigger size="sm" aria-label="Формат">
        <SelectValue />
      </SelectTrigger>
      <SelectContent>
        <SelectGroup>
          <SelectLabel>{"Форматы"}</SelectLabel>
          <SelectItem value="npm">{"npm"}</SelectItem>
          <SelectItem value="maven">{"Maven"}</SelectItem>
        </SelectGroup>
      </SelectContent>
    </Select>
  );
}

describe("Select", () => {
  it("выбор пункта показывает его в триггере", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<FormatSelect />);
    await user.click(screen.getByRole("combobox", { name: "Формат" }));

    // Act
    await user.click(await screen.findByRole("option", { name: "Maven" }));

    // Assert
    expect(screen.getByRole("combobox", { name: "Формат" })).toHaveTextContent("Maven");
  });
});
