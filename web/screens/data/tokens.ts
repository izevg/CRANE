// biome-ignore-all lint/style/noMagicNumbers: сидовые данные макета (даты, смещения, счётчики)
// i18n: не мигрирован

export type TokenRow = {
  id: string;
  name: string;
  kind: "session" | "long";
  prefix: string;
  scopes: string;
  scopeList: readonly string[];
  created: string;
  /** Дней до истечения; `null` — не истекает. */
  expiresInDays: number | null;
  /** Дней с последнего использования; `null` — не использовался. */
  lastUsedDaysAgo: number | null;
  /** Дата отзыва; у активных токенов нет. */
  revoked?: string;
};

const PURPOSES = [
  "ci-platform",
  "ci-payments",
  "ci-mobile",
  "ci-data",
  "Ноутбук Иванова",
  "Стенд stage",
  "Локальная сборка",
  "Бот релизов",
  "GitLab runner",
  "Миграция из Nexus",
  "Отладка прокси",
] as const;

const TARGETS = ["npm", "PyPI", "Maven", "NuGet"] as const;

const REPOS = ["npm-hosted", "pypi-hosted", "maven-central-proxy", "nuget-hosted", "cargo-sparse"];

const MONTHS = [
  "янв",
  "фев",
  "мар",
  "апр",
  "мая",
  "июн",
  "июл",
  "авг",
  "сен",
  "окт",
  "ноя",
  "дек",
] as const;

const EXPIRES = [3, 9, 14, 21, 30, 45, 60, 90, null, 12, 75, 5, 28, 64, 11, 88, null, 40] as const;
const LAST_USED = [0, 1, 2, 5, 34, 0, 41, 3, null, 1, 7, 0, 2, 36, 9, 0, 4, 12] as const;

const ACTIVE_COUNT = 18;
const TOTAL = 44;
const TODAY = new Date(2026, 9, 27);
const DAY_MS = 86_400_000;

function dayLabel(daysAgo: number): string {
  const date = new Date(TODAY.getTime() - daysAgo * DAY_MS);
  return `${date.getDate()} ${MONTHS[date.getMonth()]}${date.getFullYear() === 2026 ? "" : ` ${date.getFullYear()}`}`;
}

function prefixOf(index: number): string {
  const hex = ((index + 1) * 2654435761) >>> 0;
  return `crp_${hex.toString(16).padStart(8, "0")}`;
}

function buildToken(index: number): TokenRow {
  const purpose = PURPOSES[index % PURPOSES.length] ?? "ci-platform";
  const target = TARGETS[(index * 3 + Math.floor(index / PURPOSES.length)) % TARGETS.length];
  const active = index < ACTIVE_COUNT;
  const publishCount = index % 5 === 0 ? 0 : 1 + (index % 3);
  const readAll = index % 4 !== 3;
  const scopeList = [
    ...(readAll ? ["read: все репозитории"] : [`read: ${REPOS[index % REPOS.length]}`]),
    ...Array.from(
      { length: publishCount },
      (_, i) => `publish: ${REPOS[(index + i) % REPOS.length]}`,
    ),
  ];
  const base = {
    id: `tok-${index + 1}`,
    name: `${purpose} · ${target}`,
    kind: index % 3 === 1 ? ("session" as const) : ("long" as const),
    prefix: prefixOf(index),
    scopes: `read: ${readAll ? "все" : "1"} · publish: ${publishCount}`,
    scopeList,
    created: dayLabel(12 + index * 9),
    expiresInDays: active ? (EXPIRES[index] ?? null) : null,
    lastUsedDaysAgo: LAST_USED[index % LAST_USED.length] ?? null,
  };
  return active ? base : { ...base, revoked: dayLabel(1 + ((index * 5) % 60)) };
}

export const TOKENS: readonly TokenRow[] = Array.from({ length: TOTAL }, (_, i) => buildToken(i));
export const ACTIVE_TOKENS = TOKENS.filter((token) => token.revoked === undefined).length;
export const REVOKED_TOKENS = TOKENS.length - ACTIVE_TOKENS;
export const TOKEN_LIMIT = 20;
