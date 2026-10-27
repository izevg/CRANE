import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  type Access,
  cacheDistribution,
  checkCollapse,
  checkOffline,
  checkUpstreamDown,
  type Egress,
} from "./audit.ts";

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
