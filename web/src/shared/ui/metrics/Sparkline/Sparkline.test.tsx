// biome-ignore-all lint/style/noMagicNumbers: тестовые ряды — произвольные числа
import { render } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Sparkline } from "~/shared/ui/metrics";

const SVG_SELECTOR = "svg";

describe("Sparkline", () => {
  it("скрыт от скринридера", () => {
    // Arrange
    const values = [1, 3, 2];

    // Act
    const { container } = render(<Sparkline values={values} />);

    // Assert
    expect(container.querySelector(SVG_SELECTOR)).toHaveAttribute("aria-hidden", "true");
  });

  it("рисует линию и заливку по двум и более точкам", () => {
    // Arrange
    const values = [1, 3, 2];

    // Act
    const { container } = render(<Sparkline values={values} />);

    // Assert
    expect(container.querySelectorAll("path")).toHaveLength(2);
  });

  it("при одной точке рисует только пунктир-заглушку", () => {
    // Arrange
    const values = [5];

    // Act
    const { container } = render(<Sparkline values={values} />);

    // Assert
    expect(container.querySelectorAll("path")).toHaveLength(0);
    expect(container.querySelectorAll("line")).toHaveLength(1);
  });

  it("линия цели добавляется отдельной чертой", () => {
    // Arrange
    const values = [1, 3, 2];

    // Act
    const { container } = render(<Sparkline values={values} reference={2} />);

    // Assert
    expect(container.querySelectorAll("line")).toHaveLength(1);
  });
});
