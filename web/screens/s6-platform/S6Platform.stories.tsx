import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import type { Meta, StoryObj } from "@storybook/react-vite";
import {
  FlameIcon,
  FlaskConicalIcon,
  type LucideIcon,
  ShieldCheckIcon,
  Trash2Icon,
} from "lucide-react";
import { type ReactNode, useCallback, useState } from "react";
import { expect, fn, within } from "storybook/test";
import { i18n } from "~/shared/lib/i18n";
import { type Action, ActionButton, AutoRefreshToggle } from "~/shared/ui/actions";
import { PageHeader } from "~/shared/ui/layout";
import { MetricTile, ShareBar, StatChip, TimeSeriesChart } from "~/shared/ui/metrics";
import { Badge, Card, Link, ToggleGroup, ToggleGroupItem } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { PEOPLE, REPOSITORIES } from "../data/common";
import { activeFormat } from "../frame/format";
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, screenParameters } from "../frame/screen";

// ---------------------------------------------------------------- данные

const STALE_TEXT = /обновлено/;
const TB_EN = /\d\.\d TB/;
const TB = 1024 ** 4;

// i18n-fixture: названия команд — контент витрины, а не текст интерфейса
const BY_TEAM = [
  ["platform", "Платформа", 5.1],
  ["payments", "Платежи", 3.8],
  ["data", "Данные", 2.9],
  ["mobile", "Мобильная разработка", 2.2],
  ["web", "Веб-фронтенд", 1.6],
  ["ml", "ML-платформа", 1.1],
  ["infra", "Инфраструктура", 0.8],
  ["security", "Безопасность", 0.5],
  ["docs", "Документация", 0.3],
  ["labs", "Лаборатория", 0.1],
] as const;

// i18n-fixture: названия форматов — контент витрины, а не текст интерфейса
const BY_FORMAT = [
  ["npm", "npm", 7.4],
  ["oci", "OCI-образы", 5.6],
  ["maven", "Maven", 2.3],
  ["pypi", "PyPI", 1.4],
  ["nuget", "NuGet", 0.7],
  ["cargo", "Cargo", 0.4],
  ["go", "Go modules", 0.3],
  ["helm", "Helm-чарты", 0.2],
  ["gems", "RubyGems", 0.1],
  ["generic", "Generic", 0.05],
] as const;

type Dimension = "team" | "format";
function dimensions() {
  return {
    team: {
      rows: BY_TEAM,
      column: i18n._(msg`Команда`),
    },
    format: {
      rows: BY_FORMAT,
      column: i18n._(msg`Формат`),
    },
  } as const;
}

const SPARK_VOLUME = [16.9, 17.1, 17.0, 17.4, 17.6, 17.9, 18.0, 18.2, 18.4];
const SPARK_HIT = [93.1, 93.8, 94.4, 93.9, 94.6, 95.0, 94.2, 94.8, 94.2];

// 24 ч по 5 минут: детерминированный шум, чтобы снимок не плавал между прогонами.
const STEP_MS = 5 * 60 * 1000;
const POINTS = 288;
const END = Date.UTC(2026, 9, 27, 11, 0, 0);
const SLO_MS = 150;
const UPSTREAM_DOWN_FROM = 252;

function wave(index: number, period: number, seed: number): number {
  return Math.sin(index / period + seed) + 0.6 * Math.sin(index / (period / 3.7) + seed * 2);
}

function latency(base: number, amp: number, seed: number, spike: number) {
  return Array.from({ length: POINTS }, (_, index) => {
    const daily = 1 + 0.25 * Math.sin((index / POINTS) * Math.PI * 2 - 1.2);
    const outage = index >= UPSTREAM_DOWN_FROM ? spike : 0;
    const value = base * daily + amp * wave(index, 9, seed) + outage;
    return { t: END - (POINTS - 1 - index) * STEP_MS, v: Math.max(1, Math.round(value)) };
  });
}

const P50 = latency(28, 4, 0.3, 4);
const P95 = latency(88, 12, 1.1, 22);
const P99 = latency(128, 14, 2.4, 34);
const OVER_SLO = P99.filter((point) => point.v > SLO_MS).length;
const formatBytes = (bytes: number): string => activeFormat().bytes(bytes);

const timeOf = (ts: number): string =>
  new Intl.DateTimeFormat(i18n.locale, { hour: "2-digit", minute: "2-digit" }).format(ts);
const P95_PEAK = P95.reduce((best, point) => (point.v > best.v ? point : best));
const summary = () => {
  const average = activeFormat().number(Math.round(P95.reduce((sum, p) => sum + p.v, 0) / POINTS));
  const peak = activeFormat().number(P95_PEAK.v);
  const peakTime = timeOf(P95_PEAK.t);
  const overMinutes = activeFormat().number(OVER_SLO * 5);
  const totalMinutes = activeFormat().number(POINTS * 5);
  return i18n._(
    msg`Задержки за 24 ч, шаг 5 мин. p95: в среднем ${average} мс, максимум ${peak} мс в ${peakTime}. p99 выше SLO ${SLO_MS} мс ${overMinutes} мин из ${totalMinutes}.`,
  );
};

// ---------------------------------------------------------------- inline-блоки

/** Спека волны 2: §3 — entities/platform HealthList. */
type Health = {
  name: string;
  tone: "success" | "warning" | "error";
  label: string;
  caption: string;
  href: string;
};

// Сначала проблемные (patterns §3).
function components(): readonly Health[] {
  return [
    {
      name: "Upstream npmjs",
      tone: "error",
      label: i18n._(msg`Недоступен`),
      caption: i18n._(msg`с 13:42 · клиенты получают кэш до 7 дней`),
      href: "/platform/upstreams",
    },
    {
      name: i18n._(msg`LLM-шлюз`),
      tone: "warning",
      label: i18n._(msg`Деградация`),
      caption: i18n._(msg`таймаут 12 с · включена резервная модель`),
      href: "/platform/integrations",
    },
    {
      name: "PostgreSQL",
      tone: "success",
      label: i18n._(msg`Работает`),
      caption: i18n._(msg`p95 1,8 мс`),
      href: "/status",
    },
    {
      name: "Valkey",
      tone: "success",
      label: i18n._(msg`Работает`),
      caption: i18n._(msg`p95 0,4 мс`),
      href: "/status",
    },
    {
      name: "S3 main",
      tone: "success",
      label: i18n._(msg`Работает`),
      caption: i18n._(msg`p95 22 мс`),
      href: "/status",
    },
    {
      name: "Trivy",
      tone: "success",
      label: i18n._(msg`Работает`),
      caption: i18n._(msg`база v2026.10.27`),
      href: "/platform/integrations",
    },
  ];
}

function HealthList() {
  return (
    <ul className="flex flex-col">
      {components().map((item) => (
        <li key={item.name} className="border-b last:border-b-0">
          <Link
            href={item.href}
            className="flex min-h-11 items-center justify-between gap-3 py-1.5 hover:bg-accent"
          >
            <span className="flex min-w-0 flex-col">
              <span className="text-body font-semibold">{item.name}</span>
              <span className="text-caption text-muted-foreground">{item.caption}</span>
            </span>
            <StatusBadge tone={item.tone} label={item.label} />
          </Link>
        </li>
      ))}
    </ul>
  );
}

/** Спека волны 2: §32 — shared/ui/layout QuickActions. */
type Quick = { icon: LucideIcon; action: Action; caption: string };

function quick(): readonly Quick[] {
  return [
    {
      icon: FlaskConicalIcon,
      action: {
        label: i18n._(msg`Пробный прогон retention`),
        run: { onSelect: fn() },
      },
      caption: i18n._(msg`Покажет кандидатов на удаление; ничего не удаляет`),
    },
    {
      icon: Trash2Icon,
      action: {
        label: i18n._(msg`Применить retention`),
        run: { onSelect: fn() },
        stepUp: true,
      },
      caption: i18n._(msg`Удалит 1 284 версии (212 ГБ) без возможности отката`),
    },
    {
      icon: FlameIcon,
      action: {
        label: i18n._(msg`Прогреть кэш по lockfile`),
        run: { href: "/platform/readiness" },
      },
      caption: i18n._(msg`Поставит загрузку зависимостей в очередь`),
    },
    {
      icon: ShieldCheckIcon,
      action: {
        label: i18n._(msg`Проверить целостность`),
        run: { onSelect: fn() },
      },
      caption: i18n._(msg`Запустит асинхронную проверку, откроется E7`),
    },
  ];
}

function QuickActions() {
  return (
    <ul className="flex flex-col gap-3">
      {quick().map(({ icon: Icon, action, caption }) => (
        <li key={action.label} className="flex items-start gap-3">
          <Icon aria-hidden="true" className="mt-2 size-5 shrink-0 text-muted-foreground" />
          <div className="flex min-w-0 flex-col items-start gap-1">
            <ActionButton action={action} variant="secondary" />
            <span className="text-caption text-muted-foreground">{caption}</span>
          </div>
        </li>
      ))}
    </ul>
  );
}

function Panel({
  title,
  count,
  children,
  className,
}: {
  title: string;
  count?: string;
  children: ReactNode;
  className?: string;
}) {
  return (
    <Card className={className}>
      <section className="flex flex-col gap-3 px-4">
        <div className="flex items-center justify-between gap-2">
          <h2 className="text-h3">{title}</h2>
          {count ? <Badge variant="secondary">{count}</Badge> : null}
        </div>
        {children}
      </section>
    </Card>
  );
}

/** Спека волны 2: §31 — shared/ui/layout TopThreeSummary (h2 · бейдж-счётчик · ≤ 3 строки · ссылка). */
type TopRow = { id: string; title: string; caption: string; action?: Action; badge?: string };

function TopThree({
  rows,
  link,
}: {
  rows: readonly TopRow[];
  link: { href: string; label: string };
}) {
  return (
    <>
      <ul className="flex flex-col">
        {rows.map((row) => (
          <li
            key={row.id}
            className="flex items-start justify-between gap-3 border-b py-2 last:border-b-0"
          >
            <div className="flex min-w-0 flex-col">
              <span className="text-body font-semibold wrap-anywhere">{row.title}</span>
              <span className="text-caption text-muted-foreground wrap-anywhere">
                {row.caption}
              </span>
            </div>
            {row.badge ? <Badge variant="outline">{row.badge}</Badge> : null}
            {row.action ? <ActionButton action={row.action} variant="ghost" /> : null}
          </li>
        ))}
      </ul>
      <Link href={link.href} className="w-fit text-body font-semibold text-brand hover:underline">
        {link.label}
      </Link>
    </>
  );
}

function retentionRows(): readonly TopRow[] {
  const npmRepository = REPOSITORIES[0].id;
  const mavenRepository = REPOSITORIES[3].id;
  return [
    {
      id: "r1",
      title: i18n._(msg`Версии без загрузок 18 мес · ${npmRepository}`),
      caption: i18n._(msg`612 версий · 94 ГБ`),
      badge: i18n._(msg`правило 1`),
    },
    {
      id: "r2",
      title: i18n._(msg`Snapshot-сборки старше 30 дн · ${mavenRepository}`),
      caption: i18n._(msg`498 версий · 71 ГБ`),
      badge: i18n._(msg`правило 2`),
    },
    {
      id: "r3",
      title: i18n._(msg`Образы без тега старше 14 дн · oci-hosted`),
      caption: i18n._(msg`174 версии · 47 ГБ`),
      badge: i18n._(msg`правило 3`),
    },
  ];
}

const deadJobs = (): readonly TopRow[] =>
  [
    { kind: "scan.trivy", attempts: 5, error: "Timeout connecting to Trivy", id: "j-4f2a" },
    { kind: "storage.move", attempts: 3, error: "S3 PutObject: SlowDown (503)", id: "j-91cc" },
    { kind: "import.nexus", attempts: 5, error: "connection reset by peer", id: "j-07be" },
  ].map((job) => ({
    id: job.id,
    title: `${job.kind} · ${job.id}`,
    caption: i18n._(msg`попыток ${job.attempts} · Последняя ошибка: ${job.error}`),
    action: { label: i18n._(msg`Повторить ${job.id}`), run: { onSelect: fn() } },
  }));

function integrityRows(): readonly TopRow[] {
  const runBy = PEOPLE[1];
  return [
    {
      id: "i1",
      title: i18n._(msg`Последняя проверка`),
      caption: i18n._(msg`27 окт. 2026, 03:10 · 12 мин · запустил ${runBy}`),
    },
    {
      id: "i2",
      title: i18n._(msg`Проверено`),
      caption: i18n._(msg`1 % объектов: 4 812 из 481 200`),
    },
    {
      id: "i3",
      title: i18n._(msg`Следующая`),
      caption: i18n._(msg`28 окт. 2026, 03:10 по расписанию`),
    },
  ];
}

// ---------------------------------------------------------------- экран

type Mode = "normal" | "degraded";

function Tiles({ mode }: { mode: Mode }) {
  const { t } = useLingui();
  const degraded = mode === "degraded";
  return (
    <div className="grid grid-cols-1 gap-4 @xl/page:grid-cols-2 @3xl/page:grid-cols-5">
      {degraded ? (
        <MetricTile
          label={t`Объём хранилищ`}
          value={18.4}
          unit={t`ТБ`}
          delta={{ pct: 1.2, polarity: "neutral" }}
          spark={SPARK_VOLUME}
          stale={{ updatedAt: new Date(Date.now() - 3 * 60 * 1000).toISOString() }}
        />
      ) : (
        <MetricTile
          label={t`Объём хранилищ`}
          value={18.4}
          unit={t`ТБ`}
          delta={{ pct: 1.2, polarity: "neutral" }}
          spark={SPARK_VOLUME}
        />
      )}
      <MetricTile
        label={t`Попадание в кэш`}
        value={degraded ? null : 94.2}
        unit="%"
        spark={SPARK_HIT}
        reference={94}
      />
      <MetricTile
        label={t`Upstream доступны`}
        value="4/5"
        tone="warning"
        toneLabel={t`npmjs недоступен`}
        caption={t`с 13:42`}
        spark={[5, 5, 5, 5, 5, 5, 4]}
      />
      <MetricTile
        label={t`Очередь задач`}
        value={42}
        caption={t`старейшая 3 мин`}
        spark={[12, 18, 40, 35, 52, 44, 42]}
      />
      <MetricTile label={t`Последний бэкап`} value="02:14" caption={t`RPO < 5 мин`} />
    </div>
  );
}

function Usage() {
  const { t } = useLingui();
  const [dimension, setDimension] = useState<Dimension>("team");
  const { rows, column } = dimensions()[dimension];
  const pick = useCallback((next: readonly string[]) => {
    const picked = next[0];
    if (picked === "team" || picked === "format") {
      setDimension(picked);
    }
  }, []);
  return (
    <Panel title={t`Объём по командам и форматам`}>
      <ToggleGroup
        aria-label={t`Разрез`}
        size="sm"
        variant="outline"
        value={[dimension]}
        onValueChange={pick}
      >
        <ToggleGroupItem value="team">{t`По командам`}</ToggleGroupItem>
        <ToggleGroupItem value="format">{t`По форматам`}</ToggleGroupItem>
      </ToggleGroup>
      <ShareBar
        title={dimension === "team" ? t`Доли объёма по командам` : t`Доли объёма по форматам`}
        columns={{ name: column, value: t`Объём`, share: t`Доля` }}
        items={rows.map(([id, label, tb]) => ({
          id,
          label,
          value: tb * TB,
          display: activeFormat().bytes(tb * TB),
        }))}
        formatOther={formatBytes}
        caption={t`Дедупликация сэкономила 4,2 ТБ`}
      />
    </Panel>
  );
}

function Header() {
  const { t } = useLingui();
  const [window, setWindow] = useState("24h");
  const pickWindow = useCallback(
    (next: readonly string[]) => setWindow((prev) => next[0] ?? prev),
    [],
  );
  const [paused, setPaused] = useState(false);
  return (
    <PageHeader
      title={t`Платформа`}
      status={<StatusBadge tone="success" label={t`Режим: норма`} />}
      caption={t`Хранилища, кэш, upstream и фоновые задачи`}
      stats={
        <>
          <StatChip label={t`Версия`} value="0.5.0" />
          <StatChip label={t`Хранилищ`} value="4" />
          <StatChip label={t`Репозиториев`} value="18" />
        </>
      }
      aside={
        <>
          <ToggleGroup
            aria-label={t`Окно времени`}
            size="sm"
            variant="outline"
            value={[window]}
            onValueChange={pickWindow}
          >
            <ToggleGroupItem value="1h">{t`1 ч`}</ToggleGroupItem>
            <ToggleGroupItem value="6h">{t`6 ч`}</ToggleGroupItem>
            <ToggleGroupItem value="24h">{t`24 ч`}</ToggleGroupItem>
            <ToggleGroupItem value="7d">{t`7 дн`}</ToggleGroupItem>
          </ToggleGroup>
          <AutoRefreshToggle paused={paused} onPausedChange={setPaused} />
        </>
      }
    />
  );
}

function S6Platform({ mode = "normal" }: { mode?: Mode }) {
  const { t } = useLingui();
  return (
    <ScreenFrame
      active="overview"
      breadcrumbs={{
        trail: [{ label: t`Платформа`, href: "/platform" }],
        current: t`Обзор`,
      }}
    >
      <div className="flex flex-col gap-6">
        <Header />
        <Tiles mode={mode} />
        <div className="grid grid-cols-1 gap-4 @3xl/page:grid-cols-12">
          <div className="@3xl/page:col-span-8">
            <Usage />
          </div>
          <div className="@3xl/page:col-span-4">
            <Panel title={t`Состояние компонентов`} className="h-full">
              <HealthList />
            </Panel>
          </div>
          <div className="@3xl/page:col-span-8">
            <Panel title={t`Запросы и задержки`} className="h-full">
              <TimeSeriesChart
                title={t`p50 / p95 / p99 за 24 ч`}
                summary={summary()}
                unit={t`мс`}
                slo={{ value: SLO_MS, label: t`SLO ${SLO_MS} мс` }}
                series={[
                  { id: "p50", label: "p50", points: P50 },
                  { id: "p95", label: "p95", points: P95 },
                  { id: "p99", label: "p99", dash: true, points: P99 },
                ]}
              />
            </Panel>
          </div>
          <div className="@3xl/page:col-span-4">
            <Panel title={t`Быстрые действия`} className="h-full">
              <QuickActions />
            </Panel>
          </div>
          <div className="@3xl/page:col-span-4">
            <Panel
              title={t`Ближайшие удаления: 1 284 версии · 212 ГБ`}
              count={t`3 правила`}
              className="h-full"
            >
              <TopThree
                rows={retentionRows()}
                link={{ href: "/platform/retention", label: t`Настроить retention` }}
              />
            </Panel>
          </div>
          <div className="@3xl/page:col-span-4">
            <Panel title={t`Мёртвые задачи: 3`} count="3" className="h-full">
              <TopThree
                rows={deadJobs()}
                link={{ href: "/platform/queues", label: t`Открыть очередь` }}
              />
            </Panel>
          </div>
          <div className="@3xl/page:col-span-4">
            <Panel title={t`Проверка целостности: 0 ошибок, выборка 1 %`} className="h-full">
              <StatusBadge tone="success" label={t`Расхождений нет`} />
              <TopThree
                rows={integrityRows()}
                link={{ href: "/platform/integrity", label: t`Открыть проверки` }}
              />
            </Panel>
          </div>
        </div>
      </div>
    </ScreenFrame>
  );
}

const meta = {
  title: "Экраны/S6 Обзор платформы",
  component: S6Platform,
  parameters: screenParameters,
  globals: desktop,
} satisfies Meta<typeof S6Platform>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Desktop1440: Story = {
  name: "Desktop 1440, светлая",
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "Платформа" })).toBeInTheDocument();
    await expect(
      canvas.getByRole("heading", { level: 2, name: "Состояние компонентов" }),
    ).toBeInTheDocument();
  },
};

export const DesktopDark: Story = {
  name: "Desktop 1440, тёмная",
  globals: { ...desktop, theme: "dark" },
};

export const Mobile390: Story = {
  name: "Mobile 390",
  globals: mobile,
};

export const TileDataStates: Story = {
  name: "Плитки: нет данных и ошибка опроса",
  args: { mode: "degraded" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByText("Нет данных")).toBeInTheDocument();
    await expect(canvas.getByText(STALE_TEXT)).toBeInTheDocument();
  },
};

export const English: Story = {
  ...Desktop1440,
  name: "EN: подписи на английском",
  globals: { ...desktop, locale: "en" },
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    await expect(canvas.getByRole("heading", { level: 1, name: "Platform" })).toBeVisible();
    await expect(canvas.getByText("Quick actions")).toBeVisible();
    // Размеры в легенде — в формате английской локали: «5.1 TB», а не «5,1 ТБ»
    await expect(canvas.getAllByText(TB_EN).length).toBeGreaterThan(0);
  },
};
