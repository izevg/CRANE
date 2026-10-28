import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { parseArgs, parseCargo } from "./versions.ts";

const NOT_PARSED = /не разобрать/;
const UNKNOWN_TARGET = /--cargo принимает/;
const NEEDS_LATEST = /--full идёт на клиенте latest/;

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

describe("parseArgs", () => {
  it("defaults to the latest cargo without the full scenario", () => {
    // Arrange
    const argv: string[] = [];

    // Act
    const args = parseArgs(argv);

    // Assert
    assert.deepEqual(args, { which: ["latest"], full: false });
  });

  it("reads a deduplicated cargo list and the full flag", () => {
    // Arrange
    const argv = ["--cargo", "latest,previous,latest", "--full"];

    // Act
    const args = parseArgs(argv);

    // Assert
    assert.deepEqual(args, { which: ["latest", "previous"], full: true });
  });

  it("accepts the full flag alone", () => {
    // Arrange
    const argv = ["--full"];

    // Act
    const args = parseArgs(argv);

    // Assert
    assert.deepEqual(args, { which: ["latest"], full: true });
  });

  it("rejects an unknown cargo target", () => {
    // Arrange
    const argv = ["--cargo", "latest,nightly"];

    // Act
    const call = () => parseArgs(argv);

    // Assert
    assert.throws(call, UNKNOWN_TARGET);
  });

  it("rejects the full scenario without the latest client", () => {
    // Arrange
    const argv = ["--cargo", "previous", "--full"];

    // Act
    const call = () => parseArgs(argv);

    // Assert
    assert.throws(call, NEEDS_LATEST);
  });
});
