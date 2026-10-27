import { describe, expect, it } from "vitest";
import type { SearchItem } from "~/shared/api";
import { similarTo } from "./similarTo";

const typosquat = { similar_to: "@acme/ui", similar_to_purl: "pkg:npm/%40acme/ui", distance: 1 };

const item = (
  fields: Partial<Pick<SearchItem, "typosquat" | "_links">>,
): Pick<SearchItem, "typosquat" | "_links"> => ({
  _links: { self: { href: "/api/packages/x" } },
  ...fields,
});

describe("similarTo", () => {
  it("без typosquat предупреждения нет", () => {
    // Arrange
    const search = item({ typosquat: null });

    // Act
    const actual = similarTo(search);

    // Assert
    expect(actual).toBeNull();
  });

  it("со ссылкой crane:similar-to отдаёт её href API, а не маршрут", () => {
    // Arrange
    const search = item({
      typosquat,
      _links: {
        self: { href: "/x" },
        "crane:similar-to": { href: "/packages/npm/h/%40acme%2Fui" },
      },
    });

    // Act
    const actual = similarTo(search);

    // Assert
    expect(actual).toEqual({
      name: "@acme/ui",
      purl: "pkg:npm/%40acme/ui",
      apiHref: "/packages/npm/h/%40acme%2Fui",
    });
  });

  it("без ссылки настоящего пакета нет в видимых репозиториях: apiHref не задан", () => {
    // Arrange
    const search = item({ typosquat });

    // Act
    const actual = similarTo(search);

    // Assert
    expect(actual?.apiHref).toBeUndefined();
  });

  it("ссылка неверной формы игнорируется", () => {
    // Arrange
    const search = item({
      typosquat,
      _links: { self: { href: "/x" }, "crane:similar-to": "не ссылка" },
    });

    // Act
    const actual = similarTo(search);

    // Assert
    expect(actual?.apiHref).toBeUndefined();
  });
});
