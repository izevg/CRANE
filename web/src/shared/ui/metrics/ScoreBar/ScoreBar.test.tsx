import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ScoreBar } from "~/shared/ui/metrics";

const LABEL = "Релевантность эксплуатации";
const INDICATOR = "[data-tone]";

describe("ScoreBar", () => {
  it("это meter с именем, значением и текстом значения с уверенностью", () => {
    // Arrange
    const value = 0.88;
    const confidence = 0.92;

    // Act
    render(<ScoreBar label={LABEL} value={value} confidence={confidence} tone="error" />);

    // Assert
    const meter = screen.getByRole("meter", { name: LABEL });
    expect(meter).toHaveAttribute("aria-valuenow", "0.88");
    expect(meter).toHaveAttribute("aria-valuetext", "0,88, уверенность 0,92");
  });

  it("без уверенности текст значения — только оценка", () => {
    // Arrange
    const value = 0.5;

    // Act
    render(<ScoreBar label={LABEL} value={value} tone="warning" />);

    // Assert
    expect(screen.getByRole("meter")).toHaveAttribute("aria-valuetext", "0,50");
  });

  it("значение вне диапазона обрезается до 1", () => {
    // Arrange
    const value = 1.7;

    // Act
    render(<ScoreBar label={LABEL} value={value} tone="error" />);

    // Assert
    expect(screen.getByRole("meter")).toHaveAttribute("aria-valuenow", "1");
  });

  it("тон info, пришедший снаружи, попадает в data-tone полосы", () => {
    // Arrange
    const value = 0.6;

    // Act
    const { container } = render(<ScoreBar label={LABEL} value={value} tone="info" />);

    // Assert
    expect(container.querySelector(INDICATOR)).toHaveAttribute("data-tone", "info");
  });

  it("тон warning, пришедший снаружи, попадает в data-tone полосы", () => {
    // Arrange
    const value = 0.6;

    // Act
    const { container } = render(<ScoreBar label={LABEL} value={value} tone="warning" />);

    // Assert
    expect(container.querySelector(INDICATOR)).toHaveAttribute("data-tone", "warning");
  });

  it("тон error, пришедший снаружи, попадает в data-tone полосы", () => {
    // Arrange
    const value = 0.6;

    // Act
    const { container } = render(<ScoreBar label={LABEL} value={value} tone="error" />);

    // Assert
    expect(container.querySelector(INDICATOR)).toHaveAttribute("data-tone", "error");
  });

  it("тон neutral, пришедший снаружи, попадает в data-tone полосы", () => {
    // Arrange
    const value = 0.6;

    // Act
    const { container } = render(<ScoreBar label={LABEL} value={value} tone="neutral" />);

    // Assert
    expect(container.querySelector(INDICATOR)).toHaveAttribute("data-tone", "neutral");
  });

  it("пояснения рендерятся текстом", () => {
    // Arrange
    const notes = ["низкая уверенность, не влияет на политику", "не влияет на решения"];

    // Act
    render(<ScoreBar label={LABEL} value={0.9} tone="neutral" notes={notes} />);

    // Assert
    for (const note of notes) {
      expect(screen.getByText(note)).toBeInTheDocument();
    }
  });

  it("без пояснений и бейджа лишнего текста нет", () => {
    // Arrange
    const value = 0.7;

    // Act
    render(<ScoreBar label={LABEL} value={value} tone="warning" />);

    // Assert
    expect(screen.queryByText("shadow")).toBeNull();
    expect(screen.queryByText("не влияет на решения")).toBeNull();
  });

  it("бейдж выводится рядом с моделью", () => {
    // Arrange
    const badge = "shadow";

    // Act
    render(<ScoreBar label={LABEL} value={0.7} tone="neutral" badge={badge} secondary />);

    // Assert
    expect(screen.getByText(badge)).toBeInTheDocument();
  });

  it("показывает факторы, модель и уверенность", () => {
    // Arrange
    const factors = ["есть эксплойт", "сеть"];

    // Act
    render(
      <ScoreBar
        label={LABEL}
        value={0.8}
        tone="error"
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
});
