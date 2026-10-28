import { describe, expect, it } from "vitest";
import { degradedLabel } from "./labels";

describe("degradedLabel", () => {
  it.each([
    ["semantic", "Семантический поиск недоступен"],
    ["ai", "Ассистенты недоступны"],
    ["upstream:npmjs", "Upstream npmjs недоступен"],
  ])("%s — понятный текст", (code, text) => {
    // Arrange
    const input = code;

    // Act
    const label = degradedLabel(input);

    // Assert
    expect(label).toBe(text);
  });

  it("неизвестный код показывается как есть, а не теряется", () => {
    // Arrange
    const code = "scanner:lag";

    // Act
    const label = degradedLabel(code);

    // Assert
    expect(label).toBe("scanner:lag");
  });
});
