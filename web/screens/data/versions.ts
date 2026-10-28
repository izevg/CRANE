// biome-ignore-all lint/style/noMagicNumbers: детерминированные данные макетов, не продуктовый код
import type { BlockReason, HalLinks, VersionFlags, VersionState } from "~/shared/api";
import { PEOPLE } from "./common";

export type VersionRow = {
  id: string;
  version: string;
  state: VersionState;
  block_reason: BlockReason | null;
  flags: VersionFlags;
  /** ISO 8601 (UTC): одно событие — одно время в таблице, панели и ленте. */
  publishedAt: string;
  /** Человек — ФИО, робот — префикс credential и пайплайн. */
  by: { kind: "human"; name: string } | { kind: "robot"; prefix: string; pipeline: string };
  /** Источник сборки: номер MR/PR. */
  source: string;
  downloads: number;
  /** HAL элемента: ссылка `crane:block` есть только у экстренно заблокированной версии. */
  _links: HalLinks;
};

const QUARANTINED = new Set(["1.4.2", "1.3.9"]);
const BLOCKED = new Set(["1.4.0"]);
const DELETED = new Set(["1.3.4", "1.2.7", "1.2.3", "1.1.2"]);
const ROBOTS = [
  { prefix: "crr_9f8e7d6c", pipeline: "ui-release" },
  { prefix: "crr_41b0aa7e", pipeline: "ui-nightly" },
] as const;

/** Версии пакета от новых к старым: 1.4.2 … 1.0.0 (48 штук). */
const NAMES: readonly string[] = [
  ...[2, 1, 0].map((patch) => `1.4.${patch}`),
  ...Array.from({ length: 10 }, (_, index) => `1.3.${9 - index}`),
  ...Array.from({ length: 15 }, (_, index) => `1.2.${14 - index}`),
  ...Array.from({ length: 10 }, (_, index) => `1.1.${9 - index}`),
  ...Array.from({ length: 10 }, (_, index) => `1.0.${9 - index}`),
];

const MS_PER_DAY = 86_400_000;
const FIRST = Date.UTC(2026, 9, 27, 13, 42);

function stateOf(version: string): VersionState {
  if (QUARANTINED.has(version)) {
    return "quarantined";
  }
  if (BLOCKED.has(version)) {
    return "blocked";
  }
  return DELETED.has(version) ? "deleted" : "released";
}

// i18n-fixture: сообщение автора пакета об устаревании — контент витрины, а не текст интерфейса
function flagsOf(version: string): VersionFlags {
  return {
    yanked: version === "1.0.3",
    unlisted: false,
    deprecated_message: version.startsWith("1.1.") ? "Используйте 1.4.x" : null,
    dist_tags: version === "1.4.1" ? ["latest"] : version === "1.4.2" ? ["next"] : [],
    mutable: false,
  };
}

const BLOCK_ID = "0193a7c2-5e4d-7b3a-8c1f-2d9e6a4b7c10";

function reasonOf(version: string): BlockReason | null {
  return BLOCKED.has(version)
    ? {
        kind: "emergency",
        block_id: BLOCK_ID,
        advisory_id: "CVE-2026-31337",
        fixed_in: "4.17.21",
      }
    : null;
}

export const VERSION_ROWS: readonly VersionRow[] = NAMES.map((version, index) => {
  const robot = ROBOTS[index % ROBOTS.length];
  return {
    id: `v-${version}`,
    version,
    state: stateOf(version),
    block_reason: reasonOf(version),
    flags: flagsOf(version),
    publishedAt: new Date(FIRST - index * 6 * MS_PER_DAY - index * 1_137_000).toISOString(),
    by:
      index % 3 === 1 || robot === undefined
        ? { kind: "human", name: PEOPLE[index % PEOPLE.length] ?? PEOPLE[0] }
        : { kind: "robot", ...robot },
    source: `#${4521 - index * 7}`,
    downloads: 12_340 - index * 211,
    _links: {
      self: { href: `/api/packages/acme-ui/versions/${version}` },
      ...(BLOCKED.has(version)
        ? { "crane:block": { href: `/api/emergency-blocks/${BLOCK_ID}` } }
        : {}),
    },
  };
});

export const STATE_COUNTS = {
  all: VERSION_ROWS.length,
  released: VERSION_ROWS.filter((row) => row.state === "released").length,
  quarantined: VERSION_ROWS.filter((row) => row.state === "quarantined").length,
  blocked: VERSION_ROWS.filter((row) => row.state === "blocked").length,
  deleted: VERSION_ROWS.filter((row) => row.state === "deleted").length,
} as const;

export const PURL = "pkg:npm/%40acme/ui@1.4.2";
export const SHA256 = "a88d4f0e91c7b2236f5d0c8a7e4b19d3c5f6a2810b7e94d3c1f58a2b6e1d6973";
