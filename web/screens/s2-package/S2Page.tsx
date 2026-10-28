import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { useCallback } from "react";
import { InstallBlock, useInstallClient } from "~/features/install-snippet";
import { LIFECYCLE_STATE } from "~/shared/api";
import { copyToClipboard } from "~/shared/lib/core";
import { i18n } from "~/shared/lib/i18n";
import { CopyableId } from "~/shared/ui/actions";
import { DataTable, KeyValueList } from "~/shared/ui/data-display";
import { PageHeader } from "~/shared/ui/layout";
import { StatChip, TimeSeriesChart } from "~/shared/ui/metrics";
import {
  Badge,
  Card,
  CardContent,
  CardHeader,
  Link,
  Tabs,
  TabsList,
  TabsTrigger,
} from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { REPOSITORIES } from "../data/common";
import { ScreenFrame } from "../frame/ScreenFrame";
import {
  COLUMNS,
  DOWNLOADS,
  installInstructions,
  rowId,
  stale,
  TARBALL_SHA,
  versions,
} from "./S2Data";

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

function InstallSection() {
  // Страница запрашивает инструкции с предпочитаемым клиентом (или без него); показываемого клиента решает сервер
  const [preferred, setPreferred] = useInstallClient("npm");
  return (
    <InstallBlock
      preferred={preferred}
      onClientChange={setPreferred}
      instructions={installInstructions(preferred)}
      envHref="/help/environment"
    />
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

export function S2Package({ proxy = false }: { proxy?: boolean }) {
  const { t } = useLingui();
  const copy = useCallback(() => {
    void copyToClipboard(installInstructions("npm").install.code);
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
              <StatusBadge {...LIFECYCLE_STATE.active} />
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
              <StatChip label={t`Последняя`} value={<span className="font-mono">{"1.4.2"}</span>} />
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
                <TabsTrigger value="readme">{"README"}</TabsTrigger>
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
