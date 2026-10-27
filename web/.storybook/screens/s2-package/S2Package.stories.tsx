import type { Meta, StoryObj } from "@storybook/react-vite";
import { ChevronDownIcon } from "lucide-react";
import { useCallback, useState } from "react";
import { expect, within } from "storybook/test";
import { copyToClipboard } from "~/shared/lib/core";
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
  type Tone,
} from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import type { Banner } from "~/widgets/app-shell";
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
  state: { tone: Tone; label: string };
  flags: readonly string[];
  published: string;
  by: string;
};

const VERSIONS: readonly VersionRow[] = [
  {
    id: "1.4.2",
    version: "1.4.2",
    state: { tone: "success", label: "Выпущена" },
    flags: ["latest"],
    published: "27 окт 2026, 13:42",
    by: "robot:ci-publisher",
  },
  {
    id: "1.4.1",
    version: "1.4.1",
    state: { tone: "success", label: "Выпущена" },
    flags: ["stable"],
    published: "21 окт 2026, 09:15",
    by: "Петрова А.",
  },
  {
    id: "1.4.0",
    version: "1.4.0",
    state: { tone: "warning", label: "Карантин" },
    flags: ["pre-release"],
    published: "14 окт 2026, 17:03",
    by: "robot:ci-publisher",
  },
  {
    id: "1.3.9",
    version: "1.3.9",
    state: { tone: "error", label: "Заблокирована" },
    flags: ["yanked"],
    published: "02 окт 2026, 11:30",
    by: "Смирнов Д.",
  },
  {
    id: "1.3.8",
    version: "1.3.8",
    state: { tone: "success", label: "Выпущена" },
    flags: ["deprecated"],
    published: "18 сен 2026, 08:47",
    by: "Иванов И.",
  },
];

const rowId = (row: VersionRow) => row.id;

const helper = columnHelper<VersionRow>();
const COLUMNS = helper.columns([
  helper.accessor("version", {
    header: "Версия",
    meta: { priority: 1 },
    cell: (info) => <span className="font-semibold">{info.getValue()}</span>,
  }),
  helper.accessor("state", {
    header: "Состояние",
    meta: { priority: 1 },
    cell: (info) => <StatusBadge tone={info.getValue().tone} label={info.getValue().label} />,
  }),
  helper.accessor("flags", {
    header: "Флаги",
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
  helper.accessor("published", { header: "Опубликована", meta: { priority: 2 } }),
  helper.accessor("by", { header: "Кем", meta: { priority: 3 } }),
]);

const DAY = 86_400_000;
const START = Date.UTC(2026, 9, 28, 0, 0, 0) - 29 * DAY;
const DOWNLOADS = Array.from({ length: 30 }, (_, i) => ({
  t: START + i * DAY,
  v: Math.round(380 + 90 * Math.sin(i / 4) + (i % 7) * 18 + i * 6),
}));

const STALE: Banner = {
  id: "stale",
  tone: "warning",
  text: "Upstream npmjs недоступен: показаны данные от 25 окт (stale-if-error, до 7 дней)",
};

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
  const [client, setClient] = useState<Client>("npm");
  const [manual, setManual] = useState(false);
  const toggleManual = useCallback(() => setManual((value) => !value), []);
  return (
    <Section title="Установка">
      <SegmentedControl
        label="Клиент пакетов"
        items={CLIENTS}
        value={client}
        onValueChange={setClient}
      />
      <div className="flex flex-col gap-2">
        <h3 className="text-body font-semibold">Шаг 1. Однократная настройка</h3>
        <CodeBlock
          code={SETUP}
          copyLabel="Копировать команды настройки"
          label="Команды настройки"
        />
        <Button
          variant="ghost"
          size="sm"
          aria-expanded={manual}
          onClick={toggleManual}
          className="self-start"
        >
          <ChevronDownIcon aria-hidden="true" className={manual ? "rotate-180" : ""} />
          {"Настроить вручную (.npmrc)"}
        </Button>
        {manual ? (
          <>
            <CodeBlock code={NPMRC} copyLabel="Копировать строку .npmrc" label="Файл .npmrc" />
            <p className="text-caption text-muted-foreground">
              {"Токен не показывается: используйте cctl или переменную окружения."}
            </p>
          </>
        ) : null}
      </div>
      <div className="flex flex-col gap-2">
        <h3 className="text-body font-semibold">Шаг 2. Установка</h3>
        <CodeBlock
          code={INSTALL[client]}
          copyLabel="Копировать команду установки"
          label="Команда установки"
        />
      </div>
    </Section>
  );
}

function RightColumn({ proxy }: { proxy: boolean }) {
  return (
    <div className="flex flex-col gap-6">
      <Section title="Метаданные">
        <KeyValueList
          items={[
            {
              label: "PURL",
              value: <CopyableId value="pkg:npm/%40acme/ui@1.4.2" label="Скопировать PURL" />,
            },
            {
              label: "SHA-256",
              value: <CopyableId value={TARBALL_SHA} label="Скопировать SHA-256" />,
            },
            { label: "Пространство имён", value: "@acme/*" },
            {
              label: "Репозиторий",
              value: proxy ? "npm-proxy → registry.npmjs.org" : "npm-hosted",
            },
            {
              label: "Лицензия",
              value: (
                <span className="flex flex-wrap items-center gap-2">
                  {"MIT"}
                  <StatusBadge tone="success" label="Разрешена" />
                </span>
              ),
            },
            { label: "Создан", value: "12 мар 2025" },
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
      <Section title="Риски">
        <div className="flex flex-wrap items-center gap-2">
          <StatusBadge tone="warning" label="P1 · 1 находка" />
          <Badge variant="outline">{"SLA: осталось 2 дн"}</Badge>
        </div>
        <p className="text-body text-muted-foreground">
          {"Лицензии: всё разрешено. CVE-2026-31337 в зависимости lodash 4.17.20."}
        </p>
      </Section>
      {/* Спека волны 2: patterns §12 — будущий shared/ui/evidence (блок «Происхождение») */}
      <Section title="Происхождение последней версии">
        <KeyValueList
          items={[
            { label: "Пайплайн", value: "#4521" },
            {
              label: "Коммит",
              value: (
                <CopyableId
                  value="9f2c1ab4e7d03b58a1c6f20d93e47b85a0c1d2e3"
                  label="Скопировать коммит"
                />
              ),
            },
            { label: "Ref", value: "main" },
            { label: "Источник", value: <StatusBadge tone="success" label="из OIDC CI" /> },
          ]}
        />
      </Section>
    </div>
  );
}

function S2Package({ proxy = false }: { proxy?: boolean }) {
  const copy = useCallback(() => {
    void copyToClipboard(INSTALL.npm);
  }, []);
  return (
    <ScreenFrame
      active="packages"
      breadcrumbs={{ trail: [{ label: "Мои пакеты", href: "/packages" }], current: "@acme/ui" }}
      {...(proxy ? { banners: [STALE] } : {})}
    >
      <div className="flex flex-col gap-6">
        <PageHeader
          title="@acme/ui"
          status={
            <StatusBadge tone={proxy ? "warning" : "success"} label={proxy ? "Stale" : "Активен"} />
          }
          chips={
            <>
              <Badge variant="outline">{"npm"}</Badge>
              <Badge variant="outline">{proxy ? "npm-proxy" : "npm-hosted · platform"}</Badge>
            </>
          }
          caption="Компоненты интерфейса CRANE и внутренних порталов"
          stats={
            <>
              <StatChip label="Версий" value="48" />
              <StatChip label="Последняя" value="1.4.2" />
              <StatChip label="Загрузок за 30 дн" value="12 340" />
              <StatChip label="Потребителей" value="63" />
              <StatChip label="Команда" value="UI-kit" />
            </>
          }
          tabs={
            <Tabs defaultValue="overview">
              <TabsList variant="line" aria-label="Разделы пакета">
                <TabsTrigger value="overview">Обзор</TabsTrigger>
                <TabsTrigger value="versions">Версии 48</TabsTrigger>
                <TabsTrigger value="readme">README</TabsTrigger>
                <TabsTrigger value="risks">Риски 1</TabsTrigger>
                <TabsTrigger value="consumers">Потребители</TabsTrigger>
                <TabsTrigger value="settings">Настройки</TabsTrigger>
              </TabsList>
            </Tabs>
          }
          primary={{ label: "Скопировать команду", run: { onSelect: copy } }}
          secondary={[{ label: "В избранное", run: { onSelect: () => undefined } }]}
          menu={[
            { label: "Назначить замену…", run: { onSelect: () => undefined } },
            { label: "Запросить одобрение…", run: { onSelect: () => undefined } },
            { label: "Настройки", run: { href: "/packages/acme-ui/settings" } },
            {
              label: "Объявить устаревшим…",
              run: { onSelect: () => undefined },
              destructive: true,
            },
          ]}
        />
        <div className="grid grid-cols-1 gap-6 @5xl/page:grid-cols-12">
          <div className="flex min-w-0 flex-col gap-6 @5xl/page:col-span-8">
            <InstallSection />
            <Section title="Последние версии">
              <DataTable
                columns={COLUMNS}
                data={[...VERSIONS]}
                getRowId={rowId}
                caption="Последние версии @acme/ui"
              />
            </Section>
            <Section title="Динамика скачиваний">
              <TimeSeriesChart
                title="Скачивания за 30 дней"
                summary="За 30 дней: 12 340 скачиваний, рост к началу периода, максимум 912 в сутки 27 окт"
                series={[{ id: "downloads", label: "Скачивания", points: DOWNLOADS }]}
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
    await expect(within(canvasElement).getByText(STALE.text)).toBeVisible();
  },
};
