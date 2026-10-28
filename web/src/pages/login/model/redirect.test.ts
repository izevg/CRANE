import { describe, expect, it } from "vitest";
import { sanitizeRedirect } from "./redirect";

const SAFE = ["/x", "/packages/npm/react?tab=versions", "/status#top", "/"];
// имя лишь начинается как /login или /setup, но не является этим сегментом
const LOOKALIKES = ["/logins-report", "/setup-guide", "/%E0%A4%A"];
const OPEN_REDIRECTS = [
  "/\u001f/evil.example",
  "/\u007f/evil.example",
  "//evil.example",
  "//evil.example/login",
  "https://evil.example",
  "javascript:alert(1)",
  "/\\evil.example",
  "/\t/evil.example",
  "/\n/evil.example",
  "",
  "evil.example",
];
const LOGIN_OR_SETUP = [
  "/login",
  "/login?redirect=/x",
  "/login/",
  "/login#a",
  "/setup",
  "/setup/step",
  "/LOGIN",
  "/%6Cogin",
  "/./login",
  "/LOGIN/%E0%A4%A",
];
const NOT_STRINGS = [undefined, null, Number.NaN, {}, ["/x"]];

describe("sanitizeRedirect: допустимые пути", () => {
  it.each(SAFE)("%j проходит без изменений", (path) => {
    // Arrange
    const input = path;

    // Act
    const result = sanitizeRedirect(input);

    // Assert
    expect(result).toBe(path);
  });

  it.each(LOOKALIKES)("%j: не сегмент /login и /setup — проходит", (path) => {
    // Arrange
    const input = path;

    // Act
    const result = sanitizeRedirect(input);

    // Assert
    expect(result).toBe(path);
  });
});

describe("sanitizeRedirect: всё, что может стать открытым редиректом, даёт «/»", () => {
  it.each(OPEN_REDIRECTS)("%j", (input) => {
    // Arrange
    const raw = input;

    // Act
    const result = sanitizeRedirect(raw);

    // Assert
    expect(result).toBe("/");
  });

  it.each(LOGIN_OR_SETUP)("%j ведёт на вход или установку — «/»", (input) => {
    // Arrange
    const raw = input;

    // Act
    const result = sanitizeRedirect(raw);

    // Assert
    expect(result).toBe("/");
  });

  it.each(NOT_STRINGS)("не строка %j — «/»", (input) => {
    // Arrange
    const raw = input;

    // Act
    const result = sanitizeRedirect(raw);

    // Assert
    expect(result).toBe("/");
  });
});
