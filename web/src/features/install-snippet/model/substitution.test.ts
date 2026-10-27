import { describe, expect, it } from "vitest";
import { substitution } from "./substitution";

describe("substitution", () => {
  it("предпочтение и ответ различаются — подмена названа обоими клиентами", () => {
    // Arrange
    const [preferred, actual] = ["pip", "npm"] as const;

    // Act
    const result = substitution(preferred, actual, false);

    // Assert
    expect(result).toEqual({ requested: "pip", shown: "npm" });
  });

  it.each([
    ["клиенты совпадают", "npm", "npm", false],
    ["запрос ещё идёт", "pip", "npm", true],
    ["предпочтения нет", undefined, "npm", false],
    ["ответа нет", "pip", undefined, false],
  ] as const)("не подмена: %s", (_name, preferred, actual, loading) => {
    // Arrange
    // Act
    const result = substitution(preferred, actual, loading);

    // Assert
    expect(result).toBeNull();
  });
});
