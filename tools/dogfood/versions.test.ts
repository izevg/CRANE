import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { parseCargo } from "./versions.ts";

const NOT_PARSED = /не разобрать/;

describe("parseCargo", () => {
  it("extracts the version and minor line from cargo --version output", () => {
    // Arrange
    const output = "cargo 1.99.0 (abc123def 2026-10-01)\n";

    // Act
    const parsed = parseCargo(output);

    // Assert
    assert.deepEqual(parsed, { version: "1.99.0", minor: 99 });
  });

  it("keeps the patch in the version but not in the minor", () => {
    // Arrange
    const output = "cargo 1.90.3 (abc 2026-01-01)";

    // Act
    const parsed = parseCargo(output);

    // Assert
    assert.deepEqual(parsed, { version: "1.90.3", minor: 90 });
  });

  it("rejects output of another program", () => {
    // Arrange
    const output = "rustc 1.99.0 (abc 2026-10-01)";

    // Act
    const call = () => parseCargo(output);

    // Assert
    assert.throws(call, NOT_PARSED);
  });

  it("rejects an empty output", () => {
    // Arrange
    const output = "";

    // Act
    const call = () => parseCargo(output);

    // Assert
    assert.throws(call, NOT_PARSED);
  });
});
