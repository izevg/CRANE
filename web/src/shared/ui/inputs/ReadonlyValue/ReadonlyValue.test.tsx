import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { ReadonlyValue } from "~/shared/ui/inputs";

const LABEL = "Каталог данных";
const VALUE = "/var/lib/crane";

describe("ReadonlyValue", () => {
  it("значение читается как текст под именем подписи", () => {
    // Arrange
    const props = { label: LABEL, value: VALUE };

    // Act
    render(<ReadonlyValue {...props} />);

    // Assert
    expect(screen.getByRole("group", { name: LABEL })).toHaveTextContent(VALUE);
  });

  it("подпись не является label и не требует контрола", () => {
    // Arrange
    const props = { label: LABEL, value: VALUE };

    // Act
    const { container } = render(<ReadonlyValue {...props} />);

    // Assert
    expect(container.querySelector("label")).toBeNull();
    expect(screen.queryByRole("textbox")).toBeNull();
  });

  it("подсказка замка становится описанием значения", () => {
    // Arrange
    const lockHint = "Задано переменной CRANE_DATA_DIR";

    // Act
    render(<ReadonlyValue label={LABEL} value={VALUE} lockHint={lockHint} />);

    // Assert
    expect(screen.getByRole("group", { name: LABEL })).toHaveAccessibleDescription(lockHint);
  });

  it("без замка описания нет", () => {
    // Arrange
    const props = { label: LABEL, value: VALUE };

    // Act
    render(<ReadonlyValue {...props} />);

    // Assert
    expect(screen.getByRole("group", { name: LABEL })).toHaveAccessibleDescription("");
  });

  it("mono включает моноширинный шрифт у значения", () => {
    // Arrange
    const value = "CRANE_DATA_DIR";

    // Act
    render(<ReadonlyValue label={LABEL} value={value} mono />);

    // Assert
    expect(screen.getByText(value)).toHaveClass("font-mono");
  });
});
