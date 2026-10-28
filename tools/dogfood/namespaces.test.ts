import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { type Expected, type Mismatch, verify } from "./audit.ts";
import { CONFIG_ACTOR, RESERVE, SHADOW } from "./namespaces.ts";

const AGENT = "cargo/1.99.0";
const PRINCIPAL = "dogfood-1";
const OPTIONS = { principal: PRINCIPAL, loopback: ["127.0.0.1", "::1"] };
const HTTP_OK = 200;
const HTTP_NOT_FOUND = 404;
const PUBLISH_PATH = "/cargo/internal/api/v1/crates/new";
const SHADOW_PATH = "/cargo/crates-io/index/df/a_/dfa_1_99";

/** Событие 6003 без ресурса (резервирование или отказ proxy) и access-строка его запроса. */
function resourceless(
  operation: typeof RESERVE | typeof SHADOW,
  id: string,
  overrides: { code?: number } = {},
): string[] {
  const agent = `${AGENT} (abc)`;
  const reserve = operation === RESERVE;
  const audit = {
    target: "crane::audit",
    class_uid: 6003,
    "api.operation": operation,
    status: reserve ? "Success" : "Failure",
    "actor.user.uid": PRINCIPAL,
    "src_endpoint.ip": "127.0.0.1",
    "metadata.correlation_uid": id,
    "http_request.user_agent": agent,
  };
  const access = {
    target: "crane::access",
    request_id: id,
    status: overrides.code ?? (reserve ? HTTP_OK : HTTP_NOT_FOUND),
    ...(reserve ? { method: "PUT", path: PUBLISH_PATH } : { method: "GET", path: SHADOW_PATH }),
    user_agent: agent,
  };
  return [JSON.stringify(audit), JSON.stringify(access)];
}

/** Резервирование из конфигурации при старте: ни запроса, ни клиента. */
const configReserve = (): string =>
  JSON.stringify({
    target: "crane::audit",
    class_uid: 6003,
    "api.operation": RESERVE,
    status: "Success",
    "actor.user.uid": CONFIG_ACTOR,
  });

const checks = (mismatches: Mismatch[]): string[] => mismatches.map((m) => m.check);

describe("verify reservations and shadow refusals", () => {
  const expected: Expected[] = [
    { kind: "reserve", agent: AGENT },
    { kind: "shadow", agent: AGENT },
  ];

  it("accepts a reservation by publish and a shadow refusal answered with 404", () => {
    // Arrange
    const log = [...resourceless(RESERVE, "r1"), ...resourceless(SHADOW, "r2")];

    // Act
    const mismatches = verify(log.join("\n"), expected, OPTIONS);

    // Assert
    assert.deepEqual(mismatches, []);
  });

  it("rejects a shadow refusal whose request was not answered with 404", () => {
    // Arrange
    const log = [...resourceless(RESERVE, "r1"), ...resourceless(SHADOW, "r2", { code: HTTP_OK })];

    // Act
    const mismatches = verify(log.join("\n"), expected, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), ["access"]);
  });

  it("counts config reservations separately from client events", () => {
    // Arrange
    const log = [configReserve(), ...resourceless(RESERVE, "r1")];

    // Act
    const mismatches = verify(log.join("\n"), [{ kind: "reserve", agent: AGENT }], {
      ...OPTIONS,
      configReservations: 1,
    });

    // Assert
    assert.deepEqual(mismatches, []);
  });

  it("reports a missing config reservation", () => {
    // Arrange
    const log = resourceless(RESERVE, "r1");

    // Act
    const mismatches = verify(log.join("\n"), [{ kind: "reserve", agent: AGENT }], {
      ...OPTIONS,
      configReservations: 1,
    });

    // Assert
    assert.deepEqual(checks(mismatches), ["config reservations"]);
  });
});
