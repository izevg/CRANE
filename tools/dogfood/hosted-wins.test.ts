import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { indexPath } from "./hosted-wins.ts";

describe("indexPath", () => {
  it("puts a one-character name under 1/", () => {
    // Arrange
    const name = "a";

    // Act
    const actual = indexPath(name);

    // Assert
    assert.equal(actual, "1/a");
  });

  it("puts a two-character name under 2/", () => {
    // Arrange
    const name = "ab";

    // Act
    const actual = indexPath(name);

    // Assert
    assert.equal(actual, "2/ab");
  });

  it("puts a three-character name under 3/<first character>/", () => {
    // Arrange
    const name = "abc";

    // Act
    const actual = indexPath(name);

    // Assert
    assert.equal(actual, "3/a/abc");
  });

  it("splits a four-character name into two and two characters", () => {
    // Arrange
    const name = "libc";

    // Act
    const actual = indexPath(name);

    // Assert
    assert.equal(actual, "li/bc/libc");
  });

  it("splits a longer name by its first four characters and lowercases it", () => {
    // Arrange
    const name = "UrlEncoding";

    // Act
    const actual = indexPath(name);

    // Assert
    assert.equal(actual, "ur/le/urlencoding");
  });
});
