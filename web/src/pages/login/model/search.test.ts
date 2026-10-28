import { describe, expect, it } from "vitest";
import { parseLoginSearch } from "./search";

describe("parseLoginSearch", () => {
  it("известная причина и безопасный redirect проходят", () => {
    // Arrange
    const raw = { reason: "session_expired", redirect: "/status?window=24h" };

    // Act
    const search = parseLoginSearch(raw);

    // Assert
    expect(search).toEqual({ reason: "session_expired", redirect: "/status?window=24h" });
  });

  it("чужой redirect сводится к «/»", () => {
    // Arrange
    const raw = { redirect: "//evil.example" };

    // Act
    const search = parseLoginSearch(raw);

    // Assert
    expect(search).toEqual({ redirect: "/" });
  });

  it("неизвестная причина и лишние параметры отбрасываются", () => {
    // Arrange
    const raw = { reason: "reserved_internal_name", extra: "1" };

    // Act
    const search = parseLoginSearch(raw);

    // Assert
    expect(search).toEqual({});
  });

  it("redirect, который не строка, не превращается в строку", () => {
    // Arrange
    const raw = { redirect: ["/a", "/b"] };

    // Act
    const search = parseLoginSearch(raw);

    // Assert
    expect(search).toEqual({ redirect: "/" });
  });
});
