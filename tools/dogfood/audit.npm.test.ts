import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { checks, OPTIONS, PRINCIPAL } from "./audit.fixtures.ts";
import { type Expected, verify } from "./audit.ts";

const HTTP_OK = 200;
const NPM_AGENT = "npm/11.0.0";
const NPM_PURL = "pkg:npm/%40dogfood/lib-ab@1.0.0";
const NPM_PACKAGE = "/npm/internal/@dogfood%2flib-ab";

interface NpmCase {
  operation: "publish" | "delete" | "deprecate" | "tag" | "untag";
  method: string;
  path: string;
  purl: string;
  status?: string;
  http?: number;
}

/** Пара строк журнала для npm-события: аудит и access с заданным методом и путём. */
function npmPair(c: NpmCase, id: string): string[] {
  const agent = `${NPM_AGENT} node/v24.0.0 linux x64`;
  const audit = {
    target: "crane::audit",
    class_uid: 6003,
    "api.operation": c.operation,
    status: c.status ?? "Success",
    "resources.uid": c.purl,
    "actor.user.uid": PRINCIPAL,
    "src_endpoint.ip": "127.0.0.1",
    "metadata.correlation_uid": id,
    "http_request.user_agent": agent,
  };
  const access = {
    target: "crane::access",
    request_id: id,
    status: c.http ?? HTTP_OK,
    method: c.method,
    path: c.path,
    user_agent: agent,
  };
  return [JSON.stringify(audit), JSON.stringify(access)];
}

const NPM_CASES: NpmCase[] = [
  { operation: "publish", method: "PUT", path: NPM_PACKAGE, purl: NPM_PURL },
  { operation: "delete", method: "DELETE", path: `${NPM_PACKAGE}/-/x.tgz/-rev/1`, purl: NPM_PURL },
  { operation: "deprecate", method: "PUT", path: NPM_PACKAGE, purl: NPM_PURL },
  {
    operation: "tag",
    method: "PUT",
    path: "/npm/internal/-/package/x/dist-tags/stable",
    purl: NPM_PURL,
  },
  {
    operation: "untag",
    method: "DELETE",
    path: "/npm/internal/-/package/x/dist-tags/stable",
    purl: "pkg:npm/%40dogfood/lib-ab",
  },
];

describe("verify npm events", () => {
  it("accepts every npm operation with its route, agent prefix and purl shape", () => {
    // Arrange
    const log = NPM_CASES.flatMap((c, index) => npmPair(c, `n${index}`)).join("\n");
    const expected: Expected[] = NPM_CASES.map((c) => ({
      kind: "api",
      agent: NPM_AGENT,
      operation: c.operation,
      purl: c.purl,
    }));

    // Act
    const mismatches = verify(log, expected, OPTIONS);

    // Assert
    assert.deepEqual(mismatches, []);
  });

  it("matches an expected failure only to a Failure event with a non-2xx access line", () => {
    // Arrange
    const refused: NpmCase = {
      operation: "publish",
      method: "PUT",
      path: NPM_PACKAGE,
      purl: NPM_PURL,
      status: "Failure",
      http: 409,
    };
    const expected: Expected[] = [
      { kind: "api", agent: NPM_AGENT, operation: "publish", purl: NPM_PURL, outcome: "Failure" },
    ];

    // Act
    const mismatches = verify(npmPair(refused, "n1").join("\n"), expected, OPTIONS);

    // Assert
    assert.deepEqual(mismatches, []);
  });

  it("reports a Success event whose request was answered with 409", () => {
    // Arrange
    const odd: NpmCase = {
      operation: "publish",
      method: "PUT",
      path: NPM_PACKAGE,
      purl: NPM_PURL,
      http: 409,
    };
    const expected: Expected[] = [
      { kind: "api", agent: NPM_AGENT, operation: "publish", purl: NPM_PURL },
    ];

    // Act
    const mismatches = verify(npmPair(odd, "n1").join("\n"), expected, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), ["access"]);
  });

  it("reports a deprecate event that arrived over DELETE", () => {
    // Arrange
    const wrong: NpmCase = {
      operation: "deprecate",
      method: "DELETE",
      path: NPM_PACKAGE,
      purl: NPM_PURL,
    };
    const expected: Expected[] = [
      { kind: "api", agent: NPM_AGENT, operation: "deprecate", purl: NPM_PURL },
    ];

    // Act
    const mismatches = verify(npmPair(wrong, "n1").join("\n"), expected, OPTIONS);

    // Assert
    assert.deepEqual(checks(mismatches), ["access"]);
  });
});
