import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  hasDeprecationWarning,
  npmPurl,
  npmrc,
  sameTags,
  upstreamFetches,
  versionsOf,
} from "./npm.ts";

// Ссылка на переменную в .npmrc, не значение токена.
const TOKEN_REF = `\${NPM_TOKEN}`;

describe("npmPurl", () => {
  it("encodes the scope marker and omits the version for package-level events", () => {
    // Arrange
    const cases: [string, string | undefined][] = [
      ["@dogfood/lib-ab", "1.0.0"],
      ["@dogfood/lib-ab", undefined],
      ["is-number", "7.0.0"],
    ];

    // Act
    const actual = cases.map(([name, version]) => npmPurl(name, version));

    // Assert
    assert.deepEqual(actual, [
      "pkg:npm/%40dogfood/lib-ab@1.0.0",
      "pkg:npm/%40dogfood/lib-ab",
      "pkg:npm/is-number@7.0.0",
    ]);
  });
});

describe("npmrc", () => {
  it("references the token through the environment and never contains a value", () => {
    // Arrange
    const registry = "https://localhost/npm/internal/";

    // Act
    const text = npmrc(registry, { scope: "@dogfood", registry });

    // Assert
    assert.deepEqual(text.split("\n"), [
      `registry=${registry}`,
      `@dogfood:registry=${registry}`,
      `//localhost/npm/internal/:_authToken=${TOKEN_REF}`,
      `//localhost/npm/npmjs-1/:_authToken=${TOKEN_REF}`,
      "",
    ]);
  });
});

describe("sameTags", () => {
  it("accepts the exact tag set regardless of key order and rejects any difference", () => {
    // Arrange
    const want = { latest: "1.0.0", next: "1.1.0-beta.0" };
    const cases = [
      '{"next":"1.1.0-beta.0","latest":"1.0.0"}',
      '{"latest":"1.0.0"}',
      '{"latest":"1.0.0","next":"1.1.0-beta.0","stable":"1.0.0"}',
      '{"latest":"1.0.0","next":"2.0.0"}',
      "not json",
      '["latest"]',
      '[{"latest":"1.0.0","next":"1.1.0-beta.0"}]',
      '[{"latest":"1.0.0"},{"next":"1.1.0-beta.0"}]',
    ];

    // Act
    const actual = cases.map((raw) => sameTags(raw, want));

    // Assert
    assert.deepEqual(actual, [true, false, false, false, false, false, true, false]);
  });
});

describe("versionsOf", () => {
  it("reads an array, a single string and garbage", () => {
    // Arrange
    const cases = ['["1.0.0","1.1.0"]', '"1.0.0"', '["1.0.0"]', "oops", '[1,"2.0.0"]'];

    // Act
    const actual = cases.map(versionsOf);

    // Assert
    assert.deepEqual(actual, [["1.0.0", "1.1.0"], ["1.0.0"], ["1.0.0"], [], ["2.0.0"]]);
  });
});

describe("hasDeprecationWarning", () => {
  it("finds the npm warning for the exact package spec and message", () => {
    // Arrange
    const stderr = "npm warn deprecated @dogfood/lib-ab@1.0.0: dogfood: use 2.x\n";
    const cases: [string, string][] = [
      ["@dogfood/lib-ab@1.0.0", "dogfood: use 2.x"],
      ["@dogfood/lib-ab@1.1.0", "dogfood: use 2.x"],
      ["@dogfood/lib-ab@1.0.0", "other"],
    ];

    // Act
    const actual = cases.map(([spec, message]) => hasDeprecationWarning(stderr, spec, message));

    // Assert
    assert.deepEqual(actual, [true, false, false]);
  });
});

describe("upstreamFetches", () => {
  it("counts fetches of one path ignoring leading slashes", () => {
    // Arrange
    const egress = [
      { path: "is-number", status: 200, failure: undefined },
      { path: "/is-number", status: 200, failure: undefined },
      { path: "is-number/-/is-number-7.0.0.tgz", status: 200, failure: undefined },
    ];

    // Act
    const counts = ["is-number", "is-number/-/is-number-7.0.0.tgz", "other"].map((path) =>
      upstreamFetches(egress, path),
    );

    // Assert
    assert.deepEqual(counts, [2, 1, 0]);
  });
});
