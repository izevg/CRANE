// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые счётчики
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { SegmentedControl } from "~/shared/ui/inputs";

const ALL_NAME = "Все 1\u00a0000+"; // группы разрядов — неразрывный пробел

const ITEMS = [
  { value: "all", label: "Все", count: { count: 1000, capped: true } },
  { value: "released", label: "Выпущенные", count: 41 },
  { value: "held", label: "Удержанные" },
] as const;

describe("SegmentedControl", () => {
  it("выбранный сегмент помечен aria-pressed, остальные нет", () => {
    // Arrange
    const onValueChange = vi.fn();

    // Act
    render(
      <SegmentedControl
        label="Статус"
        items={ITEMS}
        value="released"
        onValueChange={onValueChange}
      />,
    );

    // Assert
    expect(screen.getByRole("button", { name: "Выпущенные 41" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: "Удержанные" })).toHaveAttribute(
      "aria-pressed",
      "false",
    );
  });

  it("усечённый счётчик входит в имя как «1 000+»", () => {
    // Arrange
    const onValueChange = vi.fn();

    // Act
    render(
      <SegmentedControl label="Статус" items={ITEMS} value="all" onValueChange={onValueChange} />,
    );

    // Assert
    expect(screen.getByRole("button", { name: ALL_NAME })).toBeInTheDocument();
  });

  it("группа получает доступное имя из label", () => {
    // Arrange
    const onValueChange = vi.fn();

    // Act
    render(
      <SegmentedControl label="Статус" items={ITEMS} value="all" onValueChange={onValueChange} />,
    );

    // Assert
    expect(screen.getByRole("group", { name: "Статус" })).toBeInTheDocument();
  });

  it("клик по другому сегменту вызывает onValueChange с его значением", async () => {
    // Arrange
    const user = userEvent.setup();
    const onValueChange = vi.fn();
    render(
      <SegmentedControl label="Статус" items={ITEMS} value="all" onValueChange={onValueChange} />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Удержанные" }));

    // Assert
    expect(onValueChange).toHaveBeenCalledWith("held");
  });

  it("клик по выбранному сегменту не снимает выбор и не вызывает onValueChange", async () => {
    // Arrange
    const user = userEvent.setup();
    const onValueChange = vi.fn();
    render(
      <SegmentedControl
        label="Статус"
        items={ITEMS}
        value="released"
        onValueChange={onValueChange}
      />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Выпущенные 41" }));

    // Assert
    expect(screen.getByRole("button", { name: "Выпущенные 41" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(onValueChange).not.toHaveBeenCalled();
  });
});
