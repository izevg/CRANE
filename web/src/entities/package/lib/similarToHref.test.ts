import { describe, expect, it } from "vitest";
import type { SearchItem } from "~/shared/api";
import { similarToHref } from "./similarToHref";

const item = (links: Record<string, unknown>): Pick<SearchItem, "_links"> => ({
  _links: { self: { href: "/api/packages/x" }, ...links },
});

describe("similarToHref", () => {
  it("со ссылкой crane:similar-to отдаёт её href API, а не маршрут SPA", () => {
    // Arrange
    const search = item({ "crane:similar-to": { href: "/packages/npm/h/%40acme%2Fui" } });

    // Act
    const actual = similarToHref(search);

    // Assert
    expect(actual).toBe("/packages/npm/h/%40acme%2Fui");
  });

  it("без ссылки настоящего пакета нет в видимых репозиториях", () => {
    // Arrange
    const search = item({});

    // Act
    const actual = similarToHref(search);

    // Assert
    expect(actual).toBeUndefined();
  });

  it("битая ссылка считается отсутствующей", () => {
    // Arrange
    const broken = [
      item({ "crane:similar-to": "не ссылка" }),
      item({ "crane:similar-to": { href: 7 } }),
    ];

    // Act
    const actual = broken.map(similarToHref);

    // Assert
    expect(actual).toEqual([undefined, undefined]);
  });
});
