import { msg } from "@lingui/core/macro";
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
import { ScreenFrame } from "../frame/ScreenFrame";
import { desktop, mobile, screenParameters } from "../frame/screen";
import { activeFormat, tr } from "../frame/tr";

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
const DIMENSIONS = {
  team: {
    rows: BY_TEAM,
    get column() {
      return tr(msg`Команда`);
    },
  },
  format: {
    rows: BY_FORMAT,
    get column() {
      return tr(msg`Формат`);
    },
  },
} as const;

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
  return tr(
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
const COMPONENTS: readonly Health[] = [
  {
    name: "Upstream npmjs",
    tone: "error",
    get label() {
      return tr(msg`Недоступен`);
    },
    get caption() {
      return tr(msg`с 13:42 · клиенты получают кэш до 7 дней`);
    },
    href: "/platform/upstreams",
  },
  {
    get name() {
      return tr(msg`LLM-шлюз`);
    },
    tone: "warning",
    get label() {
      return tr(msg`Деградация`);
    },
    get caption() {
      return tr(msg`таймаут 12 с · включена резервная модель`);
    },
    href: "/platform/integrations",
  },
  {
    name: "PostgreSQL",
    tone: "success",
    get label() {
      return tr(msg`Работает`);
    },
    get caption() {
      return tr(msg`p95 1,8 мс`);
    },
    href: "/status",
  },
  {
    name: "Valkey",
    tone: "success",
    get label() {
      return tr(msg`Работает`);
    },
    get caption() {
      return tr(msg`p95 0,4 мс`);
    },
    href: "/status",
  },
  {
    name: "S3 main",
    tone: "success",
    get label() {
      return tr(msg`Работает`);
    },
    get caption() {
      return tr(msg`p95 22 мс`);
    },
    href: "/status",
  },
  {
    name: "Trivy",
    tone: "success",
    get label() {
      return tr(msg`Работает`);
    },
    get caption() {
      return tr(msg`база v2026.10.27`);
    },
    href: "/platform/integrations",
  },
];

function HealthList() {
  return (
    <ul className="flex flex-col">
      {COMPONENTS.map((item) => (
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

const QUICK: readonly Quick[] = [
  {
    icon: FlaskConicalIcon,
    action: {
      get label() {
        return tr(msg`Пробный прогон retention`);
      },
      run: { onSelect: fn() },
    },
    get caption() {
      return tr(msg`Покажет кандидатов на удаление; ничего не удаляет`);
    },
  },
  {
    icon: Trash2Icon,
    action: {
      get label() {
        return tr(msg`Применить retention`);
      },
      run: { onSelect: fn() },
      stepUp: true,
    },
    get caption() {
      return tr(msg`Удалит 1 284 версии (212 ГБ) без возможности отката`);
    },
  },
  {
    icon: FlameIcon,
    action: {
      get label() {
        return tr(msg`Прогреть кэш по lockfile`);
      },
      run: { href: "/platform/readiness" },
    },
    get caption() {
      return tr(msg`Поставит загрузку зависимостей в очередь`);
    },
  },
  {
    icon: ShieldCheckIcon,
    action: {
      get label() {
        return tr(msg`Проверить целостность`);
      },
      run: { onSelect: fn() },
    },
    get caption() {
      return tr(msg`Запустит асинхронную проверку, откроется E7`);
    },
  },
];

function QuickActions() {
  return (
    <ul className="flex flex-col gap-3">
      {QUICK.map(({ icon: Icon, action, caption }) => (
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

const RETENTION_ROWS: readonly TopRow[] = [
  {
    id: "r1",
    get title() {
      const repository = REPOSITORIES[0].id;
      return tr(msg`Версии без загрузок 18 мес · ${repository}`);
    },
    get caption() {
      return tr(msg`612 версий · 94 ГБ`);
    },
    get badge() {
      return tr(msg`правило 1`);
    },
  },
  {
    id: "r2",
    get title() {
      const repository = REPOSITORIES[3].id;
      return tr(msg`Snapshot-сборки старше 30 дн · ${repository}`);
    },
    get caption() {
      return tr(msg`498 версий · 71 ГБ`);
    },
    get badge() {
      return tr(msg`правило 2`);
    },
  },
  {
    id: "r3",
    get title() {
      return tr(msg`Образы без тега старше 14 дн · oci-hosted`);
    },
    get caption() {
      return tr(msg`174 версии · 47 ГБ`);
    },
    get badge() {
      return tr(msg`правило 3`);
    },
  },
];

const deadJobs = (): readonly TopRow[] =>
  [
    { kind: "scan.trivy", attempts: 5, error: "Timeout connecting to Trivy", id: "j-4f2a" },
    { kind: "storage.move", attempts: 3, error: "S3 PutObject: SlowDown (503)", id: "j-91cc" },
    { kind: "import.nexus", attempts: 5, error: "connection reset by peer", id: "j-07be" },
  ].map((job) => ({
    id: job.id,
    title: `${job.kind} · ${job.id}`,
    caption: tr(msg`попыток ${job.attempts} · Последняя ошибка: ${job.error}`),
    action: { label: tr(msg`Повторить ${job.id}`), run: { onSelect: fn() } },
  }));

const INTEGRITY_ROWS: readonly TopRow[] = [
  {
    id: "i1",
    get title() {
      return tr(msg`Последняя проверка`);
    },
    get caption() {
      const runBy = PEOPLE[1];
      return tr(msg`27 окт. 2026, 03:10 · 12 мин · запустил ${runBy}`);
    },
  },
  {
    id: "i2",
    get title() {
      return tr(msg`Проверено`);
    },
    get caption() {
      return tr(msg`1 % объектов: 4 812 из 481 200`);
    },
  },
  {
    id: "i3",
    get title() {
      return tr(msg`Следующая`);
    },
    get caption() {
      return tr(msg`28 окт. 2026, 03:10 по расписанию`);
    },
  },
];

// ---------------------------------------------------------------- экран

type Mode = "normal" | "degraded";

function Tiles({ mode }: { mode: Mode }) {
  const degraded = mode === "degraded";
  return (
    <div className="grid grid-cols-1 gap-4 @xl/page:grid-cols-2 @3xl/page:grid-cols-5">
      {degraded ? (
        <MetricTile
          label={tr(msg`Объём хранилищ`)}
          value={18.4}
          unit={tr(msg`ТБ`)}
          delta={{ pct: 1.2, polarity: "neutral" }}
          spark={SPARK_VOLUME}
          stale={{ updatedAt: new Date(Date.now() - 3 * 60 * 1000).toISOString() }}
        />
      ) : (
        <MetricTile
          label={tr(msg`Объём хранилищ`)}
          value={18.4}
          unit={tr(msg`ТБ`)}
          delta={{ pct: 1.2, polarity: "neutral" }}
          spark={SPARK_VOLUME}
        />
      )}
      <MetricTile
        label={tr(msg`Попадание в кэш`)}
        value={degraded ? null : 94.2}
        unit="%"
        spark={SPARK_HIT}
        reference={94}
      />
      <MetricTile
        label={tr(msg`Upstream доступны`)}
        value="4/5"
        tone="warning"
        toneLabel={tr(msg`npmjs недоступен`)}
        caption={tr(msg`с 13:42`)}
        spark={[5, 5, 5, 5, 5, 5, 4]}
      />
      <MetricTile
        label={tr(msg`Очередь задач`)}
        value={42}
        caption={tr(msg`старейшая 3 мин`)}
        spark={[12, 18, 40, 35, 52, 44, 42]}
      />
      <MetricTile label={tr(msg`Последний бэкап`)} value="02:14" caption={tr(msg`RPO < 5 мин`)} />
    </div>
  );
}

function Usage() {
  const [dimension, setDimension] = useState<Dimension>("team");
  const { rows, column } = DIMENSIONS[dimension];
  const pick = useCallback((next: readonly string[]) => {
    const picked = next[0];
    if (picked === "team" || picked === "format") {
      setDimension(picked);
    }
  }, []);
  return (
    <Panel title={tr(msg`Объём по командам и форматам`)}>
      <ToggleGroup
        aria-label={tr(msg`Разрез`)}
        size="sm"
        variant="outline"
        value={[dimension]}
        onValueChange={pick}
      >
        <ToggleGroupItem value="team">{tr(msg`По командам`)}</ToggleGroupItem>
        <ToggleGroupItem value="format">{tr(msg`По форматам`)}</ToggleGroupItem>
      </ToggleGroup>
      <ShareBar
        title={
          dimension === "team" ? tr(msg`Доли объёма по командам`) : tr(msg`Доли объёма по форматам`)
        }
        columns={{ name: column, value: tr(msg`Объём`), share: tr(msg`Доля`) }}
        items={rows.map(([id, label, tb]) => ({
          id,
          label,
          value: tb * TB,
          display: activeFormat().bytes(tb * TB),
        }))}
        formatOther={formatBytes}
        caption={tr(msg`Дедупликация сэкономила 4,2 ТБ`)}
      />
    </Panel>
  );
}

function Header() {
  const [window, setWindow] = useState("24h");
  const pickWindow = useCallback(
    (next: readonly string[]) => setWindow((prev) => next[0] ?? prev),
    [],
  );
  const [paused, setPaused] = useState(false);
  return (
    <PageHeader
      title={tr(msg`Платформа`)}
      status={<StatusBadge tone="success" label={tr(msg`Режим: норма`)} />}
      caption={tr(msg`Хранилища, кэш, upstream и фоновые задачи`)}
      stats={
        <>
          <StatChip label={tr(msg`Версия`)} value="0.5.0" />
          <StatChip label={tr(msg`Хранилищ`)} value="4" />
          <StatChip label={tr(msg`Репозиториев`)} value="18" />
        </>
      }
      aside={
        <>
          <ToggleGroup
            aria-label={tr(msg`Окно времени`)}
            size="sm"
            variant="outline"
            value={[window]}
            onValueChange={pickWindow}
          >
            <ToggleGroupItem value="1h">{tr(msg`1 ч`)}</ToggleGroupItem>
            <ToggleGroupItem value="6h">{tr(msg`6 ч`)}</ToggleGroupItem>
            <ToggleGroupItem value="24h">{tr(msg`24 ч`)}</ToggleGroupItem>
            <ToggleGroupItem value="7d">{tr(msg`7 дн`)}</ToggleGroupItem>
          </ToggleGroup>
          <AutoRefreshToggle paused={paused} onPausedChange={setPaused} />
        </>
      }
    />
  );
}

function S6Platform({ mode = "normal" }: { mode?: Mode }) {
  return (
    <ScreenFrame
      active="overview"
      breadcrumbs={{
        trail: [{ label: tr(msg`Платформа`), href: "/platform" }],
        current: tr(msg`Обзор`),
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
            <Panel title={tr(msg`Состояние компонентов`)} className="h-full">
              <HealthList />
            </Panel>
          </div>
          <div className="@3xl/page:col-span-8">
            <Panel title={tr(msg`Запросы и задержки`)} className="h-full">
              <TimeSeriesChart
                title={tr(msg`p50 / p95 / p99 за 24 ч`)}
                summary={summary()}
                unit={tr(msg`мс`)}
                slo={{ value: SLO_MS, label: tr(msg`SLO ${SLO_MS} мс`) }}
                series={[
                  { id: "p50", label: "p50", points: P50 },
                  { id: "p95", label: "p95", points: P95 },
                  { id: "p99", label: "p99", dash: true, points: P99 },
                ]}
              />
            </Panel>
          </div>
          <div className="@3xl/page:col-span-4">
            <Panel title={tr(msg`Быстрые действия`)} className="h-full">
              <QuickActions />
            </Panel>
          </div>
          <div className="@3xl/page:col-span-4">
            <Panel
              title={tr(msg`Ближайшие удаления: 1 284 версии · 212 ГБ`)}
              count={tr(msg`3 правила`)}
              className="h-full"
            >
              <TopThree
                rows={RETENTION_ROWS}
                link={{ href: "/platform/retention", label: tr(msg`Настроить retention`) }}
              />
            </Panel>
          </div>
          <div className="@3xl/page:col-span-4">
            <Panel title={tr(msg`Мёртвые задачи: 3`)} count="3" className="h-full">
              <TopThree
                rows={deadJobs()}
                link={{ href: "/platform/queues", label: tr(msg`Открыть очередь`) }}
              />
            </Panel>
          </div>
          <div className="@3xl/page:col-span-4">
            <Panel title={tr(msg`Проверка целостности: 0 ошибок, выборка 1 %`)} className="h-full">
              <StatusBadge tone="success" label={tr(msg`Расхождений нет`)} />
              <TopThree
                rows={INTEGRITY_ROWS}
                link={{ href: "/platform/integrity", label: tr(msg`Открыть проверки`) }}
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
