import { describe, expect, it } from "vitest";
import { cx, type WithClassName } from "~/shared/lib/core";

describe("cx", () => {
  it("склеивает строки через пробел", () => {
    // Arrange
    const parts = ["flex", "gap-2"];

    // Act
    const result = cx(...parts);

    // Assert
    expect(result).toBe("flex gap-2");
  });

  it("пропускает false, null, undefined и пустые строки", () => {
    // Arrange
    const empty = "";

    // Act
    const result = cx("a", false, null, undefined, empty, "c");

    // Assert
    expect(result).toBe("a c");
  });

  it("не разрешает конфликты классов: порядок решает CSS, а не строка", () => {
    // Arrange
    const base = "p-2";

    // Act
    const result = cx(base, "p-4");

    // Assert
    expect(result).toBe("p-2 p-4");
  });
});

describe("WithClassName", () => {
  it("сохраняет дискриминированный union: поля ветки остаются доступными", () => {
    // Arrange
    type Props = WithClassName<{ kind: "icon"; label: string } | { kind: "text" }>;
    const props: Props = { kind: "icon", label: "Закрыть", className: "ml-2" };

    // Act
    const label = props.kind === "icon" ? props.label : null;

    // Assert
    expect(label).toBe("Закрыть");
  });
});
