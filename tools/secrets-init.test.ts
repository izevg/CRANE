import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, it } from "node:test";
import { initSecrets } from "./secrets-init.ts";

const ENV = {
  CRANE_TOKEN_SECRET: "./secrets/crane_token",
  POSTGRES_PASSWORD_SECRET: "./secrets/postgres_password",
  CRANE_DATABASE_URL_SECRET: "./secrets/database_url",
  POSTGRES_USER: "crane",
  POSTGRES_DB: "cranedb",
};
const MODE_MASK = 0o777;
const DIR_MODE = 0o700;
const FILE_MODE = 0o644;
const FILE_COUNT = 3;
const NAMES_VARIABLE = /POSTGRES_USER/;

let counter = 0;
const sequence = (): string => {
  counter += 1;
  return `value${counter}`;
};
const workdir = (): string => mkdtempSync(join(tmpdir(), "secrets-init-"));
const read = (root: string, name: string): string =>
  readFileSync(join(root, "secrets", name), "utf8");

describe("initSecrets", () => {
  it("creates all three files, building the URL from the password and .env names", () => {
    // Arrange
    const root = workdir();

    // Act
    const created = initSecrets(ENV, root, sequence);

    // Assert
    assert.equal(created.length, FILE_COUNT);
    assert.equal(
      read(root, "database_url"),
      `postgres://crane:${read(root, "postgres_password")}@postgres:5432/cranedb`,
    );
  });

  it("gives the directory 0700 and the files 0644", () => {
    // Arrange
    const root = workdir();

    // Act
    initSecrets(ENV, root, sequence);

    // Assert
    assert.equal(statSync(join(root, "secrets")).mode & MODE_MASK, DIR_MODE);
    assert.equal(statSync(join(root, "secrets", "crane_token")).mode & MODE_MASK, FILE_MODE);
  });

  it("never overwrites existing files and creates nothing on the second run", () => {
    // Arrange
    const root = workdir();
    initSecrets(ENV, root, sequence);
    const before = read(root, "crane_token");

    // Act
    const created = initSecrets(ENV, root, sequence);

    // Assert
    assert.deepEqual(created, []);
    assert.equal(read(root, "crane_token"), before);
  });

  it("builds a missing URL from the password that is already on disk", () => {
    // Arrange
    const root = workdir();
    initSecrets(ENV, root, sequence);
    writeFileSync(join(root, "secrets", "postgres_password"), "kept-password\n");
    rmSync(join(root, "secrets", "database_url"));

    // Act
    const created = initSecrets(ENV, root, sequence);

    // Assert
    assert.deepEqual(created, ["./secrets/database_url"]);
    assert.equal(
      read(root, "database_url"),
      "postgres://crane:kept-password@postgres:5432/cranedb",
    );
  });

  it("fails and names the variable when .env was not loaded", () => {
    // Arrange
    const env = { ...ENV, POSTGRES_USER: undefined };

    // Act
    const call = () => initSecrets(env, workdir(), sequence);

    // Assert
    assert.throws(call, { message: NAMES_VARIABLE });
  });
});
