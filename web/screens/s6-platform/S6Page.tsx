// biome-ignore-all lint/style/noMagicNumbers: числа макета (метрики, доли), не продуктовый код
import { useLingui } from "@lingui/react/macro";
import { useCallback, useState } from "react";
import { AutoRefreshToggle } from "~/shared/ui/actions";
import { PageHeader } from "~/shared/ui/layout";
import { MetricTile, ShareBar, StatChip, TimeSeriesChart } from "~/shared/ui/metrics";
import { ToggleGroup, ToggleGroupItem } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { activeFormat } from "../frame/format";
import { ScreenFrame } from "../frame/ScreenFrame";
import { HealthList, Panel, QuickActions, TopThree } from "./S6Blocks";
import {
  type Dimension,
  dimensions,
  formatBytes,
  P50,
  P95,
  P99,
  SLO_MS,
  SPARK_HIT,
  SPARK_VOLUME,
  summary,
  TB,
} from "./S6Data";
import { deadJobs, integrityRows, retentionRows } from "./S6Rows";

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

export function S6Platform({ mode = "normal" }: { mode?: Mode }) {
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
