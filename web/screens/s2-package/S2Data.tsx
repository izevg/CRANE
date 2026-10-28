// biome-ignore-all lint/style/noMagicNumbers: числа макета (размеры, даты, загрузки), не продуктовый код
import { msg } from "@lingui/core/macro";
import {
  type InstallClient,
  type InstallInstructions,
  VERSION_STATE,
  type VersionState,
} from "~/shared/api";
import { i18n } from "~/shared/lib/i18n";
import { columnHelper } from "~/shared/ui/data-display";
import { Badge } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import type { Banner } from "~/widgets/app-shell";
import { PEOPLE } from "../data/common";
import { Moment } from "../frame/Moment";

const NPM_CLIENTS: InstallClient[] = ["npm", "pnpm", "yarn"];

// i18n-fixture: заголовки шагов приходят с сервера (InstallInstructions) — контент витрины, а не текст интерфейса
/** «Сервер»: без `client` или с клиентом вне списка отдаёт клиента по умолчанию, фактический — в `client`. */
export function installInstructions(requested: InstallClient | undefined): InstallInstructions {
  const client = requested !== undefined && NPM_CLIENTS.includes(requested) ? requested : "npm";
  const install: Partial<Record<InstallClient, string>> = {
    npm: "npm i @acme/ui@1.4.2",
    pnpm: "pnpm add @acme/ui@1.4.2",
    yarn: "yarn add @acme/ui@1.4.2",
  };
  return {
    client,
    available_clients: NPM_CLIENTS,
    setup_steps: [{ title: "Вход и настройка npm", code: "cctl login\ncctl setup npm" }],
    manual_config: { path: ".npmrc", code: `//crane.example/npm/:_authToken=\${CRANE_TOKEN}` },
    install: { code: install[client] ?? "npm i @acme/ui@1.4.2" },
    _links: { self: { href: "/api/v1/packages/npm/acme-ui/install" } },
  };
}

export const TARBALL_SHA = "9b3f1c7ad04e5b82c6f01d3a7e48b9250c1f6d8e3a47b05c9d2e18f4a6b7c3d1";

type VersionRow = {
  id: string;
  version: string;
  state: VersionState;
  flags: readonly string[];
  /** ISO 8601 (UTC). */
  publishedAt: string;
  by: string;
};

export function versions(): readonly VersionRow[] {
  return [
    {
      id: "1.4.2",
      version: "1.4.2",
      state: "released",
      flags: ["latest"],
      publishedAt: "2026-10-27T13:42:00Z",
      by: "robot:ci-publisher",
    },
    {
      id: "1.4.1",
      version: "1.4.1",
      state: "released",
      flags: ["stable"],
      publishedAt: "2026-10-21T09:15:00Z",
      by: PEOPLE[1],
    },
    {
      id: "1.4.0",
      version: "1.4.0",
      state: "quarantined",
      flags: ["pre-release"],
      publishedAt: "2026-10-14T17:03:00Z",
      by: "robot:ci-publisher",
    },
    {
      id: "1.3.9",
      version: "1.3.9",
      state: "blocked",
      flags: ["yanked"],
      publishedAt: "2026-10-02T11:30:00Z",
      by: PEOPLE[2],
    },
    {
      id: "1.3.8",
      version: "1.3.8",
      state: "released",
      flags: ["deprecated"],
      publishedAt: "2026-09-18T08:47:00Z",
      by: PEOPLE[0],
    },
  ];
}

export const rowId = (row: VersionRow) => row.id;

const helper = columnHelper<VersionRow>();
export const COLUMNS = helper.columns([
  helper.accessor("version", {
    header: () => i18n._(msg`Версия`),
    meta: { priority: 1 },
    cell: (info) => <span className="font-mono font-semibold">{info.getValue()}</span>,
  }),
  helper.accessor("state", {
    header: () => i18n._(msg`Состояние`),
    meta: { priority: 1 },
    cell: (info) => <StatusBadge {...VERSION_STATE[info.getValue()]} />,
  }),
  helper.accessor("flags", {
    header: () => i18n._(msg`Флаги`),
    meta: { priority: 2 },
    cell: (info) => (
      <span className="flex gap-1">
        {info.getValue().map((flag) => (
          <Badge key={flag} variant="outline">
            {flag}
          </Badge>
        ))}
      </span>
    ),
  }),
  helper.accessor("publishedAt", {
    header: () => i18n._(msg`Опубликована`),
    meta: { priority: 2 },
    cell: (info) => <Moment iso={info.getValue()} />,
  }),
  helper.accessor("by", { header: () => i18n._(msg`Кем`), meta: { priority: 3 } }),
]);

const DAY = 86_400_000;
const START = Date.UTC(2026, 9, 28, 0, 0, 0) - 29 * DAY;
export const DOWNLOADS = Array.from({ length: 30 }, (_, i) => ({
  t: START + i * DAY,
  v: Math.round(380 + 90 * Math.sin(i / 4) + (i % 7) * 18 + i * 6),
}));

export function stale(): Banner {
  return {
    id: "stale",
    tone: "warning",
    text: i18n._(
      msg`Upstream npmjs недоступен: показаны данные от 25 окт (stale-if-error, до 7 дней)`,
    ),
  };
}
