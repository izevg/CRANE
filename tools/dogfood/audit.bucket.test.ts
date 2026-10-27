import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { checkBucketObjects, parseObjectCount } from "./audit.ts";

const FILES_IN_TREE = 112;
const BEFORE = 5;
const GREW_TO = 9;
const SHRANK_TO = 4;

describe("parseObjectCount", () => {
  it("takes the file count from the last line of fs.tree output", () => {
    // Arrange
    const output = `blobs\n└──sha256\n7 directories, ${FILES_IN_TREE} files\n`;

    // Act
    const count = parseObjectCount(output);

    // Assert
    assert.equal(count, FILES_IN_TREE);
  });

  it("returns nothing when the output has no summary line", () => {
    // Arrange
    const output = "error: directory not found\n";

    // Act
    const count = parseObjectCount(output);

    // Assert
    assert.equal(count, undefined);
  });
});

describe("checkBucketObjects", () => {
  it("accepts a non-empty bucket that did not shrink", () => {
    // Arrange
    const counts = [BEFORE, GREW_TO];

    // Act
    const problems = counts.map((after) => checkBucketObjects(BEFORE, after));

    // Assert
    assert.deepEqual(problems, [[], []]);
  });

  it("rejects an empty bucket", () => {
    // Arrange
    const empty = 0;

    // Act
    const problems = checkBucketObjects(empty, empty);

    // Assert
    assert.equal(problems.length, 1);
  });

  it("rejects a bucket that shrank across the offline phase", () => {
    // Arrange
    const shrank = SHRANK_TO;

    // Act
    const problems = checkBucketObjects(BEFORE, shrank);

    // Assert
    assert.equal(problems.length, 1);
  });
});
