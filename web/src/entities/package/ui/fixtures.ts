import type { SearchItem } from "~/shared/api";

/** Базовый результат поиска для тестов и stories: пакет без риска, typosquat и точного совпадения. */
export const BASE_ITEM: SearchItem = {
  purl_base: "pkg:npm/%40acme/ui",
  name: { display: "@acme/ui", normalized: "@acme/ui" },
  format: "npm",
  repository: { name: "npm-hosted", format: "npm", kind: "hosted", workspace: "platform" },
  description: "Дизайн-система Acme: React-компоненты, токены и темы",
  latest: { version: "1.4.2", state: "released" },
  downloads_30d: 12340,
  owner_team: "Команда UI-kit",
  last_release_at: "2026-10-24T09:15:00+03:00",
  has_readme: true,
  lifecycle: { state: "active" },
  exact_match: false,
  _links: { self: { href: "/api/packages/acme-ui" } },
};
