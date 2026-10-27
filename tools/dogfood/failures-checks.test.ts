import assert from "node:assert/strict";
import { describe, it } from "node:test";
import type { Access, Egress } from "./audit.ts";
import {
  checkCacheDegraded,
  checkDegradedFetch,
  checkNoEgress,
  checkNoFaults,
  checkNoServerErrors,
  checkServed,
  checkUnavailable,
} from "./failures-checks.ts";

const OK = 200;
const PARTIAL = 206;
const NOT_FOUND = 404;
const INTERNAL = 500;
const BAD_GATEWAY = 502;
const UNAVAILABLE = 503;
const access = (status: number, cache?: string, path = "/cargo/r/index/ab/cd/abcd"): Access => ({
  path,
  status,
  cache,
  failure: undefined,
});
const egress = (path: string): Egress => ({ path, status: 200, failure: undefined });
const ndjson = (...entries: object[]): string => entries.map((e) => JSON.stringify(e)).join("\n");
const WARN = { level: "WARN", message: "Valkey: соединение потеряно, переподключение" };
const INFO = { level: "INFO", message: "ok" };

describe("checkNoServerErrors", () => {
  it("accepts a window of 2xx and 4xx answers", () => {
    // Arrange
    const lines = [access(OK), access(NOT_FOUND)];

    // Act
    const actual = checkNoServerErrors(lines);

    // Assert
    assert.deepEqual(actual, []);
  });

  it("reports a 503 as a server error", () => {
    // Arrange
    const lines = [access(OK), access(UNAVAILABLE)];

    // Act
    const actual = checkNoServerErrors(lines);

    // Assert
    assert.deepEqual(actual, ["/cargo/r/index/ab/cd/abcd: status 503"]);
  });

  it("rejects an empty window", () => {
    // Arrange
    const lines: Access[] = [];

    // Act
    const actual = checkNoServerErrors(lines);

    // Assert
    assert.equal(actual.length, 1);
  });
});

describe("checkUnavailable", () => {
  it("accepts only 503 answers", () => {
    // Arrange
    const lines = [access(UNAVAILABLE), access(UNAVAILABLE)];

    // Act
    const actual = checkUnavailable(lines);

    // Assert
    assert.deepEqual(actual, []);
  });

  it("reports an internal error as not unavailable", () => {
    // Arrange
    const lines = [access(UNAVAILABLE), access(INTERNAL)];

    // Act
    const actual = checkUnavailable(lines);

    // Assert
    assert.deepEqual(actual, ["/cargo/r/index/ab/cd/abcd: status 500"]);
  });

  it("reports a success served during the outage", () => {
    // Arrange
    const lines = [access(OK, "hit")];

    // Act
    const actual = checkUnavailable(lines);

    // Assert
    assert.equal(actual.length, 1);
  });

  it("rejects an empty window", () => {
    // Arrange
    const lines: Access[] = [];

    // Act
    const actual = checkUnavailable(lines);

    // Assert
    assert.equal(actual.length, 1);
  });
});

describe("checkServed", () => {
  it("accepts 2xx answers", () => {
    // Arrange
    const lines = [access(OK), access(PARTIAL)];

    // Act
    const actual = checkServed(lines);

    // Assert
    assert.deepEqual(actual, []);
  });

  it("reports a 503 after recovery", () => {
    // Arrange
    const lines = [access(UNAVAILABLE)];

    // Act
    const actual = checkServed(lines);

    // Assert
    assert.equal(actual.length, 1);
  });
});

describe("checkNoEgress", () => {
  it("accepts a window without upstream fetches", () => {
    // Arrange
    const lines: Egress[] = [];

    // Act
    const actual = checkNoEgress(lines);

    // Assert
    assert.deepEqual(actual, []);
  });

  it("reports every upstream fetch", () => {
    // Arrange
    const lines = [egress("ur/ls/urlencoding")];

    // Act
    const actual = checkNoEgress(lines);

    // Assert
    assert.deepEqual(actual, ["выборка из upstream: ur/ls/urlencoding"]);
  });
});

describe("checkDegradedFetch", () => {
  it("accepts one miss answered with 200 and fetched from upstream", () => {
    // Arrange
    const lines = [access(OK, "miss")];
    const fetches = [egress("is/at/isatty")];

    // Act
    const actual = checkDegradedFetch(lines, fetches, "is/at/isatty");

    // Assert
    assert.deepEqual(actual, []);
  });

  it("reports a hit for a key that must be cold", () => {
    // Arrange
    const lines = [access(OK, "hit")];
    const fetches = [egress("is/at/isatty")];

    // Act
    const actual = checkDegradedFetch(lines, fetches, "is/at/isatty");

    // Assert
    assert.equal(actual.length, 1);
  });

  it("reports a missing upstream fetch", () => {
    // Arrange
    const lines = [access(OK, "miss")];

    // Act
    const actual = checkDegradedFetch(lines, [], "is/at/isatty");

    // Assert
    assert.equal(actual.length, 1);
  });

  it("reports a failed answer", () => {
    // Arrange
    const lines = [access(BAD_GATEWAY, "miss")];
    const fetches = [egress("is/at/isatty")];

    // Act
    const actual = checkDegradedFetch(lines, fetches, "is/at/isatty");

    // Assert
    assert.equal(actual.length, 1);
  });
});

describe("checkCacheDegraded", () => {
  it("accepts a journal with the Valkey warning and no errors", () => {
    // Arrange
    const journal = ndjson(INFO, WARN);

    // Act
    const actual = checkCacheDegraded(journal);

    // Assert
    assert.deepEqual(actual, []);
  });

  it("reports a journal without the Valkey warning", () => {
    // Arrange
    const journal = ndjson(INFO);

    // Act
    const actual = checkCacheDegraded(journal);

    // Assert
    assert.equal(actual.length, 1);
  });

  it("reports an ERROR-level line", () => {
    // Arrange
    const journal = ndjson(WARN, { level: "ERROR", message: "boom" });

    // Act
    const actual = checkCacheDegraded(journal);

    // Assert
    assert.equal(actual.length, 1);
  });

  it("reports a panic printed as plain text", () => {
    // Arrange
    const journal = `${ndjson(WARN)}\nthread 'tokio-runtime-worker' panicked at src/x.rs:1:1`;

    // Act
    const actual = checkCacheDegraded(journal);

    // Assert
    assert.equal(actual.length, 1);
  });
});

describe("checkNoFaults", () => {
  it("accepts a journal without the Valkey warning", () => {
    // Arrange
    const journal = ndjson(INFO);

    // Act
    const actual = checkNoFaults(journal);

    // Assert
    assert.deepEqual(actual, []);
  });

  it("reports an ERROR-level line", () => {
    // Arrange
    const journal = ndjson(INFO, { level: "ERROR", message: "boom" });

    // Act
    const actual = checkNoFaults(journal);

    // Assert
    assert.equal(actual.length, 1);
  });

  it("reports a panic printed as plain text", () => {
    // Arrange
    const journal = "thread 'main' panicked at src/x.rs:1:1";

    // Act
    const actual = checkNoFaults(journal);

    // Assert
    assert.equal(actual.length, 1);
  });
});
