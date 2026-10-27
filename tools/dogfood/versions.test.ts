import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { minorOf, parseCargoVersion, previousMinor } from "./versions.ts";

const NOT_PARSED = /не разобрать/;

describe("previousMinor", () => {
  it("returns the previous minor line of a stable release", () => {
    // Arrange
    const version = "1.99.0";

    // Act
    const previous = previousMinor(version);

    // Assert
    assert.equal(previous, "1.98");
  });

  it("ignores the patch component", () => {
    // Arrange
    const version = "1.90.3";

    // Act
    const previous = previousMinor(version);

    // Assert
    assert.equal(previous, "1.89");
  });

  it("rejects strings that are not a Rust release", () => {
    // Arrange
    const garbage = "latest";

    // Act
    const call = () => previousMinor(garbage);

    // Assert
    assert.throws(call, NOT_PARSED);
  });

  it("rejects the first minor line, which has no predecessor", () => {
    // Arrange
    const version = "1.0.0";

    // Act
    const call = () => previousMinor(version);

    // Assert
    assert.throws(call, NOT_PARSED);
  });
});

describe("parseCargoVersion", () => {
  it("extracts the release from cargo --version output", () => {
    // Arrange
    const output = "cargo 1.99.0 (abc123def 2026-10-01)\n";

    // Act
    const version = parseCargoVersion(output);

    // Assert
    assert.equal(version, "1.99.0");
  });

  it("rejects output of another program", () => {
    // Arrange
    const output = "rustc 1.99.0 (abc 2026-10-01)";

    // Act
    const call = () => parseCargoVersion(output);

    // Assert
    assert.throws(call, NOT_PARSED);
  });
});

describe("minorOf", () => {
  it("drops the patch component", () => {
    // Arrange
    const version = "1.99.2";

    // Act
    const minor = minorOf(version);

    // Assert
    assert.equal(minor, "1.99");
  });
});
