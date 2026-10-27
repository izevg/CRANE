import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  type Access,
  accessLines,
  cacheDistribution,
  checkCollapse,
  checkOffline,
  checkUpstreamDown,
  type Egress,
  type Expected,
  egressLines,
  leaks,
  type Mismatch,
  verify,
} from "./audit.ts";

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
  { kind: "authFailure", agent: AGENT },
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

describe("accessLines", () => {
  it("возвращает cache и upstream_failure строк access и пропускает остальные", () => {
    // Arrange
    const ndjson = [
      JSON.stringify({
        target: "crane::access",
        path: "/cargo/crates-io/index/se/rd/serde",
        status: 200,
        cache: "miss",
      }),
      JSON.stringify({
        target: "crane::access",
        path: "/cargo/crates-io/api/v1/crates/serde/1.0.0/download",
        status: 502,
        upstream_failure: "digest-mismatch",
      }),
      JSON.stringify({ target: "crane::audit", class_uid: 3002 }),
      "не json",
    ].join("\n");

    // Act
    const actual = accessLines(ndjson);

    // Assert
    assert.deepEqual(actual, [
      {
        path: "/cargo/crates-io/index/se/rd/serde",
        status: 200,
        cache: "miss",
        failure: undefined,
      },
      {
        path: "/cargo/crates-io/api/v1/crates/serde/1.0.0/download",
        status: 502,
        cache: undefined,
        failure: "digest-mismatch",
      },
    ]);
  });
});

describe("egressLines", () => {
  it("возвращает путь, статус и отказ строк egress и пропускает остальные", () => {
    // Arrange
    const ndjson = [
      JSON.stringify({
        target: "crane::upstream",
        host: "index.crates.io",
        path: "fe/rr/ferris-says",
        status: 200,
        elapsed_ms: 40,
      }),
      JSON.stringify({ target: "crane::upstream", path: "se/rd/serde", failure: "unavailable" }),
      JSON.stringify({ target: "crane::access", path: "/cargo/x/index/se/rd/serde", status: 200 }),
      "не json",
    ].join("\n");

    // Act
    const actual = egressLines(ndjson);

    // Assert
    assert.deepEqual(actual, [
      { path: "fe/rr/ferris-says", status: 200, failure: undefined },
      { path: "se/rd/serde", status: undefined, failure: "unavailable" },
    ]);
  });
});

const BAD_GATEWAY = 502;
const UNAUTHORIZED = 401;
const OK = 200;
const SERVICE_UNAVAILABLE = 503;
const LAST_OK = 499;
const FIRST_SERVER_ERROR = 500;
const HIT_SEEN = /cache=miss.*200\/hit/;
const COLLAPSE = {
  accessPath: "/cargo/r/index/fe/rr/ferris-says",
  upstreamPath: "fe/rr/ferris-says",
  requests: 3,
};
const served = (cache: string | undefined, status = 200, path = COLLAPSE.accessPath): Access => ({
  path,
  status,
  cache,
  failure: undefined,
});
const fetched = (path = COLLAPSE.upstreamPath): Egress => ({
  path,
  status: 200,
  failure: undefined,
});

describe("checkCollapse", () => {
  it("accepts all requests missing the cache with exactly one upstream fetch", () => {
    // Arrange
    const access = [served("miss"), served("miss"), served("miss")];
    const egress = [fetched(), fetched("se/rd/serde")];

    // Act
    const problems = checkCollapse(access, egress, COLLAPSE);

    // Assert
    assert.deepEqual(problems, []);
  });

  it("rejects a request that was served from the cache (no overlap)", () => {
    // Arrange
    const access = [served("miss"), served("miss"), served("hit")];

    // Act
    const problems = checkCollapse(access, [fetched()], COLLAPSE);

    // Assert
    assert.equal(problems.length, 1);
    assert.match(problems[0] ?? "", HIT_SEEN);
  });

  it("rejects two upstream fetches of the key", () => {
    // Arrange
    const access = [served("miss"), served("miss"), served("miss")];

    // Act
    const problems = checkCollapse(access, [fetched(), fetched()], COLLAPSE);

    // Assert
    assert.deepEqual(problems, ["выборок из upstream 2, ожидалась ровно одна"]);
  });

  it("rejects a failed request and a wrong request count", () => {
    // Arrange
    const access = [served("miss"), served("miss", BAD_GATEWAY)];

    // Act
    const problems = checkCollapse(access, [fetched()], COLLAPSE);

    // Assert
    assert.equal(problems.length, 2);
  });

  it("rejects the absence of any upstream fetch", () => {
    // Arrange
    const access = [served("miss"), served("miss"), served("miss")];

    // Act
    const problems = checkCollapse(access, [], COLLAPSE);

    // Assert
    assert.deepEqual(problems, ["выборок из upstream 0, ожидалась ровно одна"]);
  });
});

describe("checkOffline", () => {
  it("accepts responses served from the cache in any freshness zone", () => {
    // Arrange
    const lines = [served("hit"), served("stale"), served("stale-on-error")];

    // Act
    const problems = checkOffline(lines);

    // Assert
    assert.deepEqual(problems, []);
  });

  it("rejects a miss", () => {
    // Arrange
    const lines = [served("hit"), served("miss")];

    // Act
    const problems = checkOffline(lines);

    // Assert
    assert.equal(problems.length, 1);
  });

  it("rejects a 5xx response even from the cache", () => {
    // Arrange
    const lines = [served("stale-on-error", BAD_GATEWAY)];

    // Act
    const problems = checkOffline(lines);

    // Assert
    assert.equal(problems.length, 1);
  });

  it("rejects status 500 and accepts 499", () => {
    // Arrange
    const lines = [served("hit", LAST_OK), served("hit", FIRST_SERVER_ERROR)];

    // Act
    const problems = checkOffline(lines);

    // Assert
    assert.equal(problems.length, 1);
  });

  it("rejects a line without cache", () => {
    // Arrange
    const lines = [served(undefined)];

    // Act
    const problems = checkOffline(lines);

    // Assert
    assert.equal(problems.length, 1);
  });

  it("does not require cache on the index config but rejects its 5xx", () => {
    // Arrange
    const config = "/cargo/r/index/config.json";
    const lines = [
      served("hit"),
      served(undefined, UNAUTHORIZED, config),
      served(undefined, BAD_GATEWAY, config),
    ];

    // Act
    const problems = checkOffline(lines);

    // Assert
    assert.deepEqual(problems, [`${config}: status ${BAD_GATEWAY}, cache -`]);
  });

  it("rejects a phase made only of index config requests", () => {
    // Arrange
    const lines = [served(undefined, OK, "/cargo/r/index/config.json")];

    // Act
    const problems = checkOffline(lines);

    // Assert
    assert.equal(problems.length, 1);
  });

  it("rejects an empty phase", () => {
    // Arrange
    const lines: Access[] = [];

    // Act
    const problems = checkOffline(lines);

    // Assert
    assert.equal(problems.length, 1);
  });
});

describe("cacheDistribution", () => {
  it("counts lines per cache value", () => {
    // Arrange
    const lines = [served("hit"), served("hit"), served("stale"), served(undefined)];

    // Act
    const distribution = cacheDistribution(lines);

    // Assert
    assert.deepEqual(distribution, { hit: 2, stale: 1, "-": 1 });
  });
});

describe("checkUpstreamDown", () => {
  const probeLine = (status: number, failure: string | undefined): Access => ({
    path: "/cargo/r/index/co/ws/cowsay",
    status,
    cache: undefined,
    failure,
  });
  const refusedFetch: Egress = { path: "co/ws/cowsay", status: undefined, failure: "unavailable" };

  it("accepts a 502 probe with an unavailable fetch and no successful fetches", () => {
    // Arrange
    const probe = [probeLine(BAD_GATEWAY, "unavailable")];

    // Act
    const problems = checkUpstreamDown(probe, [refusedFetch]);

    // Assert
    assert.deepEqual(problems, []);
  });

  it("rejects a successful upstream fetch after the cut-off", () => {
    // Arrange
    const probe = [probeLine(BAD_GATEWAY, "unavailable")];
    const egress = [refusedFetch, fetched()];

    // Act
    const problems = checkUpstreamDown(probe, egress);

    // Assert
    assert.equal(problems.length, 1);
  });

  it("rejects a probe that was answered instead of failing", () => {
    // Arrange
    const probe = [probeLine(OK, undefined)];

    // Act
    const problems = checkUpstreamDown(probe, [refusedFetch]);

    // Assert
    assert.equal(problems.length, 1);
  });

  it("rejects a probe that failed for another reason", () => {
    // Arrange
    const probe = [probeLine(BAD_GATEWAY, "refused")];

    // Act
    const problems = checkUpstreamDown(probe, [refusedFetch]);

    // Assert
    assert.equal(problems.length, 1);
  });

  it("rejects a probe answered with another status than 502", () => {
    // Arrange
    const probe = [probeLine(SERVICE_UNAVAILABLE, "unavailable")];

    // Act
    const problems = checkUpstreamDown(probe, [refusedFetch]);

    // Assert
    assert.equal(problems.length, 1);
  });

  it("rejects the absence of any failed fetch", () => {
    // Arrange
    const probe = [probeLine(BAD_GATEWAY, "unavailable")];

    // Act
    const problems = checkUpstreamDown(probe, []);

    // Assert
    assert.deepEqual(problems, ["в egress-журнале нет ни одной неудачной выборки"]);
  });

  it("rejects a missing probe line", () => {
    // Arrange
    const probe: Access[] = [];

    // Act
    const problems = checkUpstreamDown(probe, [refusedFetch]);

    // Assert
    assert.deepEqual(problems, ["access-строк пробы 0, ожидалась одна"]);
  });
});
