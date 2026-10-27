import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { parse } from "smol-toml";
import { publishable } from "./overlay.ts";

const INTERNAL = new Set(["crane-domain", "crane-ports"]);
const VERSION = "0.2.0";
const REGISTRY = "crane";

// smol-toml отдаёт объекты без прототипа: для сравнения приводим к обычным.
const plain = (manifest: string): unknown => JSON.parse(JSON.stringify(parse(manifest)));

const overlay = (manifest: string): unknown =>
  plain(publishable(manifest, INTERNAL, VERSION, REGISTRY));

describe("publishable", () => {
  it("turns an internal path dependency into a registry dependency of the workspace version", () => {
    // Arrange
    const manifest = '[dependencies]\ncrane-domain = { path = "../domain" }\n';

    // Act
    const result = overlay(manifest);

    // Assert
    assert.deepEqual(result, {
      dependencies: {
        "crane-domain": { path: "../domain", version: VERSION, registry: REGISTRY },
      },
    });
  });

  it("leaves external dependencies untouched", () => {
    // Arrange
    const manifest = [
      "[dependencies]",
      'serde = "1"',
      'local-helper = { path = "../helper" }',
      'tokio = { version = "1", features = ["net"] }',
      "",
    ].join("\n");

    // Act
    const result = overlay(manifest);

    // Assert
    assert.deepEqual(result, plain(manifest));
  });

  it("rewrites dev, build and target-specific tables as well", () => {
    // Arrange
    const manifest = [
      '[dev-dependencies]\ncrane-ports = { path = "../ports" }',
      '[build-dependencies]\ncrane-domain = { path = "../domain" }',
      "[target.'cfg(unix)'.dependencies]\ncrane-ports = { path = \"../ports\" }",
      "",
    ].join("\n\n");
    const rewritten = { path: "", version: VERSION, registry: REGISTRY };

    // Act
    const result = overlay(manifest);

    // Assert
    assert.deepEqual(result, {
      "dev-dependencies": { "crane-ports": { ...rewritten, path: "../ports" } },
      "build-dependencies": { "crane-domain": { ...rewritten, path: "../domain" } },
      target: {
        "cfg(unix)": { dependencies: { "crane-ports": { ...rewritten, path: "../ports" } } },
      },
    });
  });

  it("restricts publishing of the workspace root to the registry", () => {
    // Arrange
    const manifest = '[workspace]\nmembers = ["a"]\n\n[workspace.package]\npublish = false\n';

    // Act
    const result = overlay(manifest);

    // Assert
    assert.deepEqual(result, {
      workspace: { members: ["a"], package: { publish: [REGISTRY] } },
    });
  });

  it("matches an internal crate by its real package name when the dependency is renamed", () => {
    // Arrange
    const manifest = [
      "[dependencies]",
      'domain = { package = "crane-domain", path = "../domain" }',
      'other = { package = "serde", path = "../serde" }',
      "",
    ].join("\n");

    // Act
    const result = overlay(manifest);

    // Assert
    assert.deepEqual(result, {
      dependencies: {
        domain: {
          package: "crane-domain",
          path: "../domain",
          version: VERSION,
          registry: REGISTRY,
        },
        other: { package: "serde", path: "../serde" },
      },
    });
  });

  it("does not touch dependencies inherited from the workspace", () => {
    // Arrange
    const manifest = "[dependencies]\ncrane-domain = { workspace = true }\n";

    // Act
    const result = overlay(manifest);

    // Assert
    assert.deepEqual(result, plain(manifest));
  });
});
