import { describe, expect, it } from "vitest";
import { type HrefKind, hrefKind } from "./hrefKind";

const CASES: [string, string, HrefKind][] = [
  ["путь приложения", "/status?window=24h", "internal"],
  ["относительный путь", "packages/npm/left-pad", "internal"],
  ["https", "https://docs.example/crane", "external"],
  ["http в верхнем регистре", "HTTP://docs.example", "external"],
  ["// без схемы", "//cdn.example/x", "external"],
  ["обратная косая вместо //", "/\\evil.example", "external"],
  ["mailto", "mailto:admin@example.com", "mailto"],
  ["javascript", "javascript:alert(1)", "blocked"],
  ["JAVASCRIPT в верхнем регистре", "JAVASCRIPT:alert(1)", "blocked"],
  ["пробелы перед схемой", "  javascript:alert(1)", "blocked"],
  ["управляющий символ перед схемой", "\u0001javascript:alert(1)", "blocked"],
  ["табуляция внутри схемы", "java\tscript:alert(1)", "blocked"],
  ["перевод строки внутри схемы", "java\nscript:alert(1)", "blocked"],
  ["data", "data:text/html,<script>alert(1)</script>", "blocked"],
  ["vbscript", "vbscript:msgbox(1)", "blocked"],
  ["неизвестная схема", "ftp://files.example/x", "blocked"],
  ["пробелы перед //", "\t //evil.example", "external"],
];

describe("hrefKind", () => {
  it.each(CASES)("%s: %j", (_name, href, expected) => {
    // Arrange
    const input = href;

    // Act
    const kind = hrefKind(input);

    // Assert
    expect(kind).toBe(expected);
  });
});
