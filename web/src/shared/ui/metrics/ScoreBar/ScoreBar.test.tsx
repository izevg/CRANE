// biome-ignore-all lint/style/noMagicNumbers: граничные оценки и уверенности — произвольные числа из 0..1
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ScoreBar } from "~/shared/ui/metrics";

const INDICATOR = "[data-tone]";
const LABEL = "Релевантность эксплуатации";
const LOW_NOTE = "низкая уверенность, не влияет на политику";
const SHADOW_NOTE = "не влияет на решения";

describe("ScoreBar", () => {
  it("это meter с именем, значением и текстом значения с уверенностью", () => {
    // Arrange
    const value = 0.88;
    const confidence = 0.92;

    // Act
    render(<ScoreBar label={LABEL} value={value} confidence={confidence} />);

    // Assert
    const meter = screen.getByRole("meter", { name: LABEL });
    expect(meter).toHaveAttribute("aria-valuenow", "0.88");
    expect(meter).toHaveAttribute("aria-valuetext", "0,88, уверенность 0,92");
  });

  it("без уверенности текст значения — только оценка", () => {
    // Arrange
    const value = 0.5;

    // Act
    render(<ScoreBar label={LABEL} value={value} />);

    // Assert
    expect(screen.getByRole("meter")).toHaveAttribute("aria-valuetext", "0,50");
  });

  it("значение вне диапазона обрезается до 1", () => {
    // Arrange
    const value = 1.7;

    // Act
    render(<ScoreBar label={LABEL} value={value} />);

    // Assert
    expect(screen.getByRole("meter")).toHaveAttribute("aria-valuenow", "1");
  });

  it("при уверенности 0,4 показывает подпись о низкой уверенности", () => {
    // Arrange
    const confidence = 0.4;

    // Act
    render(<ScoreBar label={LABEL} value={0.9} confidence={confidence} />);

    // Assert
    expect(screen.getByText(LOW_NOTE)).toBeInTheDocument();
  });

  it("на границе 0,5 уверенность ещё достаточна: подписи нет", () => {
    // Arrange
    const confidence = 0.5;

    // Act
    render(<ScoreBar label={LABEL} value={0.9} confidence={confidence} />);

    // Assert
    expect(screen.queryByText(LOW_NOTE)).toBeNull();
  });

  it("при уверенности 0,49 подпись о низкой уверенности есть", () => {
    // Arrange
    const confidence = 0.49;

    // Act
    render(<ScoreBar label={LABEL} value={0.9} confidence={confidence} />);

    // Assert
    expect(screen.getByText(LOW_NOTE)).toBeInTheDocument();
  });

  it("при уверенности 0,6 подписи о низкой уверенности нет", () => {
    // Arrange
    const confidence = 0.6;

    // Act
    render(<ScoreBar label={LABEL} value={0.9} confidence={confidence} />);

    // Assert
    expect(screen.queryByText(LOW_NOTE)).toBeNull();
  });

  it("shadow-оценка помечена бейджем и подписью «не влияет на решения»", () => {
    // Arrange
    const shadow = true;

    // Act
    render(<ScoreBar label={LABEL} value={0.7} shadow={shadow} />);

    // Assert
    expect(screen.getByText("shadow")).toBeInTheDocument();
    expect(screen.getByText(SHADOW_NOTE)).toBeInTheDocument();
  });

  it("обычная оценка не помечается как shadow", () => {
    // Arrange
    const value = 0.7;

    // Act
    render(<ScoreBar label={LABEL} value={value} />);

    // Assert
    expect(screen.queryByText(SHADOW_NOTE)).toBeNull();
  });

  it("показывает факторы, модель и уверенность", () => {
    // Arrange
    const factors = ["есть эксплойт", "сеть"];

    // Act
    render(
      <ScoreBar
        label={LABEL}
        value={0.8}
        confidence={0.92}
        factors={factors}
        model="systemone v3.1"
      />,
    );

    // Assert
    expect(screen.getAllByRole("listitem")).toHaveLength(factors.length);
    expect(screen.getByText("модель systemone v3.1")).toBeInTheDocument();
    expect(screen.getByText("уверенность 0,92")).toBeInTheDocument();
  });

  it.each([
    [0.49, "info"],
    [0.5, "warning"],
    [0.79, "warning"],
    [0.8, "error"],
  ])("оценка %s даёт тон полосы %s", (value, expected) => {
    // Arrange
    const confidence = 0.9;

    // Act
    const { container } = render(<ScoreBar label={LABEL} value={value} confidence={confidence} />);

    // Assert
    expect(container.querySelector(INDICATOR)).toHaveAttribute("data-tone", expected);
  });

  it("при низкой уверенности полоса приглушена независимо от оценки", () => {
    // Arrange
    const confidence = 0.4;

    // Act
    const { container } = render(<ScoreBar label={LABEL} value={0.9} confidence={confidence} />);

    // Assert
    expect(container.querySelector(INDICATOR)).toHaveAttribute("data-tone", "neutral");
  });

  it("у shadow-оценки полоса приглушена", () => {
    // Arrange
    const shadow = true;

    // Act
    const { container } = render(<ScoreBar label={LABEL} value={0.9} shadow={shadow} />);

    // Assert
    expect(container.querySelector(INDICATOR)).toHaveAttribute("data-tone", "neutral");
  });
});
