import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { accessLines, egressLines, leaks } from "./audit.ts";

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
