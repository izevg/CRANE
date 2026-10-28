import { msg } from "@lingui/core/macro";
import { ActivityIcon } from "lucide-react";
import { describe, expect, it } from "vitest";
import type { HalLinks } from "~/shared/api";
import { type NavSpec, navGroups } from "./navItems";

const SPEC: readonly NavSpec[] = [
  {
    rel: "crane:status",
    to: "/status",
    label: msg`Статус системы`,
    icon: ActivityIcon,
    group: "platform",
  },
  { rel: "crane:search", to: "/search", label: msg`Поиск`, icon: ActivityIcon, group: "platform" },
];

const allRoutes = { hasRoute: () => true, isActive: () => false };

function links(...rels: string[]): HalLinks {
  return {
    self: { href: "/me" },
    ...Object.fromEntries(rels.map((rel) => [rel, { href: `/${rel}` }])),
  };
}

describe("navGroups: меню из _links", () => {
  it("показывает пункты, для которых есть ссылка в _links и маршрут", () => {
    // Arrange
    const me = links("crane:status", "crane:search");

    // Act
    const groups = navGroups(me, allRoutes, SPEC);

    // Assert
    expect(groups[0]?.items.map((item) => item.href)).toEqual(["/status", "/search"]);
  });

  it("раздел без ссылки в _links не показывается (роль не разрешает)", () => {
    // Arrange
    const me = links("crane:status");

    // Act
    const groups = navGroups(me, allRoutes, SPEC);

    // Assert
    expect(groups[0]?.items.map((item) => item.id)).toEqual(["crane:status"]);
  });

  it("ссылка есть, а маршрута в роутере нет — пункта нет (иначе вёл бы на 404)", () => {
    // Arrange
    const me = links("crane:status", "crane:search");

    // Act
    const groups = navGroups(
      me,
      { hasRoute: (to) => to === "/status", isActive: () => false },
      SPEC,
    );

    // Assert
    expect(groups[0]?.items.map((item) => item.href)).toEqual(["/status"]);
  });

  it("ссылка с неизвестным rel в меню не попадает", () => {
    // Arrange
    const me = links("crane:unknown");

    // Act
    const groups = navGroups(me, allRoutes, SPEC);

    // Assert
    expect(groups).toEqual([]);
  });

  it("активность пункта берётся из совпадения маршрута", () => {
    // Arrange
    const me = links("crane:status", "crane:search");

    // Act
    const groups = navGroups(
      me,
      { hasRoute: () => true, isActive: (to) => to === "/search" },
      SPEC,
    );

    // Assert
    expect(groups[0]?.items.map((item) => item.active)).toEqual([false, true]);
  });

  it("подписи пунктов и группы берутся из каталога сообщений", () => {
    // Arrange
    const me = links("crane:status");

    // Act
    const groups = navGroups(me, allRoutes, SPEC);

    // Assert
    expect(groups[0]).toMatchObject({ label: "Платформа", items: [{ label: "Статус системы" }] });
  });
});
