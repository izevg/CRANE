import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  A_010,
  A_011,
  AGENT,
  API_EXPECTED,
  api,
  authFailure,
  checks,
  goodLog,
  OPTIONS,
} from "./audit.fixtures.ts";
import { verify } from "./audit.ts";

describe("verify", () => {
  it("accepts a log that matches the expected events exactly", () => {
    // Arrange
    const log = goodLog().join("\n");

    // Act
    const mismatches = verify(log, API_EXPECTED, OPTIONS);

    // Assert
    assert.deepEqual(mismatches, []);
  });

  it("reports a duplicated event as a count mismatch", () => {
    // Arrange
    const log = [...goodLog(), ...api("publish", A_010, "r3")].join("\n");

    // Act
    const mismatches = verify(log, API_EXPECTED, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), [`6003 ${AGENT}`]);
    assert.ok(mismatches[0]?.detail.endsWith("ожидалось 1, в журнале 2"));
  });

  it("reports a missing event", () => {
    // Arrange
    const log = [...api("publish", A_010, "r1"), authFailure()].join("\n");

    // Act
    const mismatches = verify(log, API_EXPECTED, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), [`6003 ${AGENT}`]);
    assert.equal(mismatches[0]?.detail, `yank ${A_011} Success: ожидалось 1, в журнале 0`);
  });

  it("reports a failed publish as a mismatch, not as the expected success", () => {
    // Arrange
    const log = [
      ...api("publish", A_010, "r1", { status: "Failure" }),
      ...api("yank", A_011, "r2"),
      authFailure(),
    ].join("\n");

    // Act
    const mismatches = verify(log, API_EXPECTED, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), [`6003 ${AGENT}`, `6003 ${AGENT}`, "access"]);
  });

  it("reports an actor other than the configured principal", () => {
    // Arrange
    const log = [
      ...api("publish", A_010, "r1", { uid: "crane" }),
      ...api("yank", A_011, "r2"),
      authFailure(),
    ].join("\n");

    // Act
    const mismatches = verify(log, API_EXPECTED, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), ["principal"]);
  });

  it("reports a client address that is not loopback (proxy trust is missing)", () => {
    // Arrange
    const log = [
      ...api("publish", A_010, "r1", { ip: "10.250.1.2" }),
      ...api("yank", A_011, "r2"),
      authFailure(),
    ].join("\n");

    // Act
    const mismatches = verify(log, API_EXPECTED, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), ["src_endpoint"]);
  });

  it("reports an audit event without a matching access line", () => {
    // Arrange
    const log = [
      ...api("publish", A_010, "r1", { correlation: "unknown" }),
      ...api("yank", A_011, "r2"),
      authFailure(),
    ].join("\n");

    // Act
    const mismatches = verify(log, API_EXPECTED, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), ["correlation"]);
  });

  it("reports an access line whose method does not belong to the operation", () => {
    // Arrange
    const [audit = "", access = ""] = api("yank", A_011, "r2");
    const wrong = JSON.stringify({ ...JSON.parse(access), method: "PUT" });
    const log = [...api("publish", A_010, "r1"), audit, wrong, authFailure()].join("\n");

    // Act
    const mismatches = verify(log, API_EXPECTED, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), ["access"]);
  });

  it("reports a user agent of another cargo version as an unknown client", () => {
    // Arrange
    const log = [
      ...api("publish", A_010, "r1", { agent: "cargo/1.98.0 (abc)" }),
      ...api("yank", A_011, "r2"),
      authFailure(),
    ].join("\n");

    // Act
    const mismatches = verify(log, API_EXPECTED, OPTIONS);

    // Assert
    assert.ok(checks(mismatches).includes("agent"));
    assert.ok(checks(mismatches).includes(`6003 ${AGENT}`));
  });

  it("reports a missing authentication failure", () => {
    // Arrange
    const log = [...api("publish", A_010, "r1"), ...api("yank", A_011, "r2")].join("\n");

    // Act
    const mismatches = verify(log, API_EXPECTED, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), [`3002 ${AGENT}`]);
  });

  it("reports a line that is not JSON and ignores empty lines", () => {
    // Arrange
    const log = ["", ...goodLog(), "panic: oops", ""].join("\n");

    // Act
    const mismatches = verify(log, API_EXPECTED, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), ["json"]);
  });
});
