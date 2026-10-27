import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { StatChip } from "~/shared/ui/metrics";

const CHIP_SELECTOR = "[data-slot=stat-chip]";

describe("StatChip", () => {
  it("читается одной строкой «подпись значение»", () => {
    // Arrange
    const FULL_TEXT = "Версий 48";

    // Act
    render(<StatChip label="Версий" value="48" />);

    // Assert
    expect(
      screen.getByText((_, element) => element?.textContent === FULL_TEXT, {
        selector: CHIP_SELECTOR,
      }),
    ).toBeInTheDocument();
  });

  it("принимает узел в качестве значения", () => {
    // Arrange
    const VERSION = "1.4.2";
    const value = <code>{VERSION}</code>;

    // Act
    render(<StatChip label="Последняя" value={value} />);

    // Assert
    expect(screen.getByText("1.4.2")).toBeInTheDocument();
  });

  it("не является ссылкой", () => {
    // Arrange
    const label = "Версий";

    // Act
    render(<StatChip label={label} value="48" />);

    // Assert
    expect(screen.queryByRole("link")).toBeNull();
  });
});
