import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import { ChevronDownIcon } from "lucide-react";
import { useCallback, useState } from "react";
import { expect, within } from "storybook/test";
import { LIFECYCLE_STATE, type VersionState } from "~/shared/api";
import { copyToClipboard } from "~/shared/lib/core";
import { i18n } from "~/shared/lib/i18n";
import { CopyableId } from "~/shared/ui/actions";
import { CodeBlock, columnHelper, DataTable, KeyValueList } from "~/shared/ui/data-display";
import { SegmentedControl } from "~/shared/ui/inputs";
import { PageHeader } from "~/shared/ui/layout";
import { StatChip, TimeSeriesChart } from "~/shared/ui/metrics";
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardHeader,
  Link,
  Tabs,
  TabsList,
  TabsTrigger,
} from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import type { Banner } from "~/widgets/app-shell";
import { PEOPLE, REPOSITORIES } from "../data/common";
import { versionBadge } from "../data/states";
import { Moment } from "../frame/Moment";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, screenParameters } from "../frame/screen";

type Client = "npm" | "pnpm" | "yarn";

const CLIENTS = [
  { value: "npm", label: "npm" },
  { value: "pnpm", label: "pnpm" },
  { value: "yarn", label: "yarn" },
] as const;

const INSTALL: Record<Client, string> = {
  npm: "npm i @acme/ui@1.4.2",
  pnpm: "pnpm add @acme/ui@1.4.2",
  yarn: "yarn add @acme/ui@1.4.2",
};

const SETUP = "cctl login\ncctl setup npm";
const NPMRC = `//crane.example/npm/:_authToken=\${CRANE_TOKEN}`;
const TARBALL_SHA = "9b3f1c7ad04e5b82c6f01d3a7e48b9250c1f6d8e3a47b05c9d2e18f4a6b7c3d1";

type VersionRow = {
  id: string;
  version: string;
  state: VersionState;
  flags: readonly string[];
  /** ISO 8601 (UTC). */
  publishedAt: string;
  by: string;
};

function versions(): readonly VersionRow[] {
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

const rowId = (row: VersionRow) => row.id;

const helper = columnHelper<VersionRow>();
const COLUMNS = helper.columns([
  helper.accessor("version", {
    header: () => i18n._(msg`Версия`),
    meta: { priority: 1 },
    cell: (info) => <span className="font-mono font-semibold">{info.getValue()}</span>,
  }),
  helper.accessor("state", {
    header: () => i18n._(msg`Состояние`),
    meta: { priority: 1 },
    cell: (info) => <StatusBadge {...versionBadge(info.getValue())} />,
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
const DOWNLOADS = Array.from({ length: 30 }, (_, i) => ({
  t: START + i * DAY,
  v: Math.round(380 + 90 * Math.sin(i / 4) + (i % 7) * 18 + i * 6),
}));

function stale(): Banner {
  return {
    id: "stale",
    tone: "warning",
    text: i18n._(
      msg`Upstream npmjs недоступен: показаны данные от 25 окт (stale-if-error, до 7 дней)`,
    ),
  };
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <Card>
      <CardHeader>
        <h2 className="text-h3">{title}</h2>
      </CardHeader>
      <CardContent className="flex flex-col gap-3">{children}</CardContent>
    </Card>
  );
}

// Спека волны 2: patterns §7 — будущий shared/ui/install-block (InstallBlock)
function InstallSection() {
  const { t } = useLingui();
  const [client, setClient] = useState<Client>("npm");
  const [manual, setManual] = useState(false);
  const toggleManual = useCallback(() => setManual((value) => !value), []);
  return (
    <Section title={t`Установка`}>
      <SegmentedControl
        label={t`Клиент пакетов`}
        items={CLIENTS}
        value={client}
        onValueChange={setClient}
      />
      <div className="flex flex-col gap-2">
        <h3 className="text-body font-semibold">{t`Шаг 1. Однократная настройка`}</h3>
        <CodeBlock
          code={SETUP}
          copyLabel={t`Копировать команды настройки`}
          label={t`Команды настройки`}
        />
        <Button
          variant="ghost"
          size="sm"
          aria-expanded={manual}
          onClick={toggleManual}
          className="self-start"
        >
          <ChevronDownIcon aria-hidden="true" className={manual ? "rotate-180" : ""} />
          {t`Настроить вручную (.npmrc)`}
        </Button>
        {manual ? (
          <>
            <CodeBlock
              code={NPMRC}
              copyLabel={t`Копировать строку .npmrc`}
              label={t`Файл .npmrc`}
            />
            <p className="text-caption text-muted-foreground">
              {t`Токен не показывается: используйте cctl или переменную окружения.`}
            </p>
          </>
        ) : null}
      </div>
      <div className="flex flex-col gap-2">
        <h3 className="text-body font-semibold">{t`Шаг 2. Установка`}</h3>
        <CodeBlock
          code={INSTALL[client]}
          copyLabel={t`Копировать команду установки`}
          label={t`Команда установки`}
        />
      </div>
    </Section>
  );
}

function RightColumn({ proxy }: { proxy: boolean }) {
  const { t } = useLingui();
  return (
    <div className="flex flex-col gap-6">
      <Section title={t`Метаданные`}>
        <KeyValueList
          items={[
            {
              label: "PURL",
              value: (
                <CopyableId
                  marked={false}
                  value="pkg:npm/%40acme/ui@1.4.2"
                  label={t`Скопировать PURL`}
                />
              ),
            },
            {
              label: "SHA-256",
              value: <CopyableId value={TARBALL_SHA} label={t`Скопировать SHA-256`} />,
            },
            { label: t`Пространство имён`, value: "@acme/*" },
            {
              label: t`Репозиторий`,
              value: proxy ? `${REPOSITORIES[1].id} → registry.npmjs.org` : REPOSITORIES[0].id,
            },
            {
              label: t`Лицензия`,
              value: (
                <span className="flex flex-wrap items-center gap-2">
                  {"MIT"}
                  <StatusBadge tone="success" label={t`Разрешена`} />
                </span>
              ),
            },
            { label: t`Создан`, value: t`12 мар 2025` },
            {
              label: "GitLab",
              value: (
                <Link
                  href="https://gitlab.example/platform/ui"
                  className="underline underline-offset-3"
                >
                  {"platform/ui"}
                </Link>
              ),
            },
          ]}
        />
      </Section>
      {/* Спека волны 2: patterns §10 — будущий features/risk-summary (панель «Риски» с приоритетом и SLA) */}
      <Section title={t`Риски`}>
        <div className="flex flex-wrap items-center gap-2">
          <StatusBadge tone="warning" label={t`P1 · 1 находка`} />
          <Badge variant="outline">{t`SLA: осталось 2 дн`}</Badge>
        </div>
        <p className="text-body text-muted-foreground">
          {t`Лицензии: всё разрешено. CVE-2026-31337 в зависимости lodash 4.17.20.`}
        </p>
      </Section>
      {/* Спека волны 2: patterns §12 — будущий shared/ui/evidence (блок «Происхождение») */}
      <Section title={t`Происхождение последней версии`}>
        <KeyValueList
          items={[
            { label: t`Пайплайн`, value: "#4521" },
            {
              label: t`Коммит`,
              value: (
                <CopyableId
                  value="9f2c1ab4e7d03b58a1c6f20d93e47b85a0c1d2e3"
                  label={t`Скопировать коммит`}
                />
              ),
            },
            { label: "Ref", value: "main" },
            {
              label: t`Источник`,
              value: <StatusBadge tone="success" label={t`из OIDC CI`} />,
            },
          ]}
        />
      </Section>
    </div>
  );
}

function S2Package({ proxy = false }: { proxy?: boolean }) {
  const { t } = useLingui();
  const copy = useCallback(() => {
    void copyToClipboard(INSTALL.npm);
  }, []);
  return (
    <ScreenFrame
      active="packages"
      breadcrumbs={{
        trail: [{ label: t`Мои пакеты`, href: "/packages" }],
        current: "@acme/ui",
      }}
      {...(proxy ? { banners: [stale()] } : {})}
    >
      <div className="flex flex-col gap-6">
        <PageHeader
          title="@acme/ui"
          status={
            proxy ? (
              <StatusBadge tone="warning" label="Stale" />
            ) : (
              <StatusBadge
                tone={LIFECYCLE_STATE.active.tone}
                label={t(LIFECYCLE_STATE.active.label)}
              />
            )
          }
          chips={
            <>
              <Badge variant="outline">{"npm"}</Badge>
              <Badge variant="outline">
                {proxy ? REPOSITORIES[1].id : `${REPOSITORIES[0].id} · platform`}
              </Badge>
            </>
          }
          caption={t`Компоненты интерфейса CRANE и внутренних порталов`}
          stats={
            <>
              <StatChip label={t`Версий`} value="48" />
              <StatChip label={t`Последняя`} value={<span className="font-mono">1.4.2</span>} />
              <StatChip label={t`Загрузок за 30 дн`} value="12 340" />
              <StatChip label={t`Потребителей`} value="63" />
              <StatChip label={t`Команда`} value="UI-kit" />
            </>
          }
          tabs={
            <Tabs defaultValue="overview">
              <TabsList variant="line" aria-label={t`Разделы пакета`}>
                <TabsTrigger value="overview">{t`Обзор`}</TabsTrigger>
                <TabsTrigger value="versions">{t`Версии 48`}</TabsTrigger>
                <TabsTrigger value="readme">README</TabsTrigger>
                <TabsTrigger value="risks">{t`Риски 1`}</TabsTrigger>
                <TabsTrigger value="consumers">{t`Потребители`}</TabsTrigger>
                <TabsTrigger value="settings">{t`Настройки`}</TabsTrigger>
              </TabsList>
            </Tabs>
          }
          primary={{ label: t`Скопировать команду`, run: { onSelect: copy } }}
          secondary={[{ label: t`В избранное`, run: { onSelect: () => undefined } }]}
          menu={[
            { label: t`Назначить замену…`, run: { onSelect: () => undefined } },
            { label: t`Запросить одобрение…`, run: { onSelect: () => undefined } },
            { label: t`Настройки`, run: { href: "/packages/acme-ui/settings" } },
            {
              label: t`Объявить устаревшим…`,
              run: { onSelect: () => undefined },
              destructive: true,
            },
          ]}
        />
        <div className="grid grid-cols-1 gap-6 @5xl/page:grid-cols-12">
          <div className="flex min-w-0 flex-col gap-6 @5xl/page:col-span-8">
            <InstallSection />
            <Section title={t`Последние версии`}>
              <DataTable
                columns={COLUMNS}
                data={[...versions()]}
                getRowId={rowId}
                caption={t`Последние версии @acme/ui`}
              />
            </Section>
            <Section title={t`Динамика скачиваний`}>
              <TimeSeriesChart
                title={t`Скачивания за 30 дней`}
                summary={i18n._(
                  msg`За 30 дней: 12 340 скачиваний, рост к началу периода, максимум 912 в сутки 27 окт`,
                )}
                series={[{ id: "downloads", label: t`Скачивания`, points: DOWNLOADS }]}
                area
              />
            </Section>
          </div>
          <div className="min-w-0 @5xl/page:col-span-4">
            <RightColumn proxy={proxy} />
          </div>
        </div>
      </div>
    </ScreenFrame>
  );
}

const meta = {
  title: "Экраны/S2 Карточка пакета",
  component: S2Package,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof S2Package>;

export default meta;
type Story = StoryObj<typeof meta>;

export const DesktopLight: Story = {
  name: "Desktop, светлая",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "@acme/ui" })).toBeVisible();
    await expect(canvas.getByRole("region", { name: "Команда установки" })).toHaveTextContent(
      "npm i @acme/ui@1.4.2",
    );
  },
};

export const DesktopDark: Story = {
  name: "Desktop, тёмная",
  globals: { ...desktop, theme: "dark" },
};

export const Mobile: Story = {
  name: "Mobile 390",
  globals: mobile,
};

export const ProxyStale: Story = {
  name: "Proxy: stale-баннер",
  args: { proxy: true },
  play: async ({ canvasElement }) => {
    // Текст баннера дублируется в live-регионе (announce), поэтому совпадений может быть два
    const [banner] = await within(canvasElement).findAllByText(stale().text);
    await expect(banner).toBeVisible();
  },
};

export const English: Story = {
  ...DesktopLight,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { name: "Install" })).toBeVisible();
    await expect(canvas.getByRole("navigation", { name: "Primary menu" })).toBeVisible();
  },
};
