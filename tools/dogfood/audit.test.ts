import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { type Expected, leaks, type Mismatch, verify } from "./audit.ts";

const AGENT = "cargo/1.99.0";
const PRINCIPAL = "dogfood-1";
const OPTIONS = { principal: PRINCIPAL, loopback: ["127.0.0.1", "::1"] };
const A_010 = "pkg:cargo/dfa_1_99@0.1.0";
const A_011 = "pkg:cargo/dfa_1_99@0.1.1";

interface Overrides {
  uid?: string;
  ip?: string;
  agent?: string;
  correlation?: string;
  status?: string;
}

const ROUTE = {
  publish: { method: "PUT", path: "/cargo/internal/api/v1/crates/new" },
  yank: { method: "DELETE", path: "/cargo/internal/api/v1/crates/dfa_1_99/0.1.1/yank" },
  unyank: { method: "PUT", path: "/cargo/internal/api/v1/crates/dfa_1_99/0.1.1/unyank" },
} as const;

/** Пара строк журнала: событие 6003 и его access-строка. */
function api(
  operation: keyof typeof ROUTE,
  purl: string,
  id: string,
  overrides: Overrides = {},
): string[] {
  const agent = overrides.agent ?? `${AGENT} (abc)`;
  const audit = {
    target: "crane::audit",
    class_uid: 6003,
    "api.operation": operation,
    status: overrides.status ?? "Success",
    "resources.uid": purl,
    "actor.user.uid": overrides.uid ?? PRINCIPAL,
    "src_endpoint.ip": overrides.ip ?? "127.0.0.1",
    "metadata.correlation_uid": overrides.correlation ?? id,
    "http_request.user_agent": agent,
  };
  const access = {
    target: "crane::access",
    request_id: id,
    status: 200,
    ...ROUTE[operation],
    user_agent: agent,
  };
  return [JSON.stringify(audit), JSON.stringify(access)];
}

const authFailure = (agent = `${AGENT} (abc)`): string =>
  JSON.stringify({
    target: "crane::audit",
    class_uid: 3002,
    status: "Failure",
    "http_request.user_agent": agent,
  });

const API_EXPECTED: Expected[] = [
  { kind: "api", agent: AGENT, operation: "publish", purl: A_010 },
  { kind: "api", agent: AGENT, operation: "yank", purl: A_011 },
  { kind: "authFailure", agent: AGENT, min: 1 },
];

const goodLog = (): string[] => [
  ...api("publish", A_010, "r1"),
  ...api("yank", A_011, "r2"),
  authFailure(),
];

const checks = (mismatches: Mismatch[]): string[] => mismatches.map((m) => m.check);

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
    assert.deepEqual(checks(mismatches), [`6003 ${AGENT}`, `6003 ${AGENT}`]);
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

describe("leaks", () => {
  it("returns the names of secrets found in the logs", () => {
    // Arrange
    const raw = "GET /index token=s3cr3t-value done";
    const secrets = [
      { name: "токен", value: "s3cr3t-value" },
      { name: "пароль", value: "other-value" },
    ];

    // Act
    const found = leaks(raw, secrets);

    // Assert
    assert.deepEqual(found, ["токен"]);
  });

  it("returns nothing when no secret appears", () => {
    // Arrange
    const raw = "GET /index done";
    const secrets = [{ name: "токен", value: "s3cr3t-value" }];

    // Act
    const found = leaks(raw, secrets);

    // Assert
    assert.deepEqual(found, []);
  });

  it("does not treat an empty secret as present everywhere", () => {
    // Arrange
    const raw = "anything";
    const secrets = [{ name: "пусто", value: "" }];

    // Act
    const found = leaks(raw, secrets);

    // Assert
    assert.deepEqual(found, []);
  });
});
