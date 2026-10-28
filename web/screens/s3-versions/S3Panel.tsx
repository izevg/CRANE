// biome-ignore-all lint/style/noMagicNumbers: числа макета (размеры, даты), не продуктовый код
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { type ReactNode, useCallback, useState } from "react";
import { VersionStatus } from "~/entities/version";
import { i18n } from "~/shared/lib/i18n";
import { ActionButton, CopyableId } from "~/shared/ui/actions";
import { DiffViewer, JsonViewer, KeyValueList, Timeline } from "~/shared/ui/data-display";
import { DetailPanel } from "~/shared/ui/layout";
import {
  Callout,
  CalloutDescription,
  CalloutTitle,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Tabs,
  TabsList,
  TabsTrigger,
} from "~/shared/ui/primitives";
import { PURL, SHA256 } from "../data/versions";
import { activeFormat } from "../frame/format";
import { Moment } from "../frame/Moment";
import { noop, SELECTED_AT } from "./S3Columns";

function Section({
  title,
  aside,
  children,
}: {
  title: string;
  aside?: ReactNode;
  children: ReactNode;
}) {
  return (
    <section className="flex flex-col gap-3">
      <div className="flex items-center justify-between gap-2">
        <h3 className="text-h3">{title}</h3>
        {aside}
      </div>
      {children}
    </section>
  );
}

const lifecycle = () =>
  [
    {
      id: "published",
      time: SELECTED_AT,
      title: i18n._(msg`Опубликована`),
      actor: "crr_9f8e7d6c · ui-release",
      source: "pipeline #4521",
    },
    {
      id: "sha",
      time: "2026-10-27T13:42:10Z",
      title: i18n._(msg`SHA-256 подтверждён`),
      detail: <CopyableId value={SHA256} label={i18n._(msg`Скопировать SHA-256`)} />,
    },
    {
      id: "scan",
      time: "2026-10-27T13:48:00Z",
      title: i18n._(msg`Скан: найдена 1 уязвимость`),
      tone: "warning",
      status: {
        tone: "warning",
        label: i18n._(msg`Карантин`),
      },
      source: "Trivy",
      link: { label: "CVE-2026-31337", run: { href: "/findings/CVE-2026-31337" } },
    },
    {
      id: "gates",
      time: "2026-10-27T13:49:00Z",
      title: i18n._(msg`Гейт безопасности: отказ`),
      tone: "error",
      status: {
        tone: "error",
        label: i18n._(msg`Не пройден`),
      },
    },
    {
      id: "request",
      time: "2026-10-27T14:07:00Z",
      title: i18n._(msg`Запрос на продвижение в release`),
      tone: "info",
      status: {
        tone: "info",
        label: i18n._(msg`Ждёт AppSec`),
      },
      actor: i18n._(msg`Иванов И.`),
    },
  ] as const;

const DIFF_ROWS = [
  { kind: "same", path: "name", text: '"name": "@acme/ui"' },
  { kind: "change", path: "version", before: '"1.4.1"', after: '"1.4.2"' },
  { kind: "change", path: "dependencies.lodash", before: '"^4.17.20"', after: '"^4.17.21"' },
  { kind: "same", path: "dependencies.react", text: '"react": "^19.0.0"' },
] as const;

const FILES = [
  ["package.json", 2_104],
  ["dist/index.js", 98_311],
  ["dist/index.d.ts", 14_872],
  ["dist/components/DataTable.js", 31_450],
  ["dist/components/DetailPanel.js", 18_907],
  ["README.md", 4_216],
  ["LICENSE", 11_357],
] as const;

const METADATA = {
  name: "@acme/ui",
  version: "1.4.2",
  license: "Apache-2.0",
  dist: {
    tarball: "https://crane.acme.example/npm/@acme/ui/-/ui-1.4.2.tgz",
    integrity: "sha512-9fR2kQvXr7LmT0aB5nHcYe1uWdZs8GjP",
    unpackedSize: 188_416,
  },
  dependencies: { lodash: "^4.17.21", react: "^19.0.0" },
  keywords: ["ui", "design-system", "react"],
  deprecated: null,
};

// Спека волны 2: S3 «Сводка» (screens.md) — будущий widget версии `VersionSummary`.
function SummaryTab() {
  const { t } = useLingui();
  return (
    <div className="flex flex-col gap-6">
      <Callout tone="warning">
        <CalloutTitle>{t`Продвижение в release заблокировано: 2 причины`}</CalloutTitle>
        <CalloutDescription>
          {t`Находка CVE-2026-31337 (P1) в lodash 4.17.20 · нет одобрения AppSec.`}
        </CalloutDescription>
      </Callout>
      <KeyValueList
        items={[
          {
            label: "PURL",
            value: <CopyableId marked={false} value={PURL} full label={t`Скопировать PURL`} />,
          },
          { label: t`Опубликована`, value: <Moment iso={SELECTED_AT} /> },
          { label: t`Кем`, value: "crr_9f8e7d6c · ui-release" },
          { label: t`Размер`, value: activeFormat().bytes(188_416) },
          {
            label: "SHA-256",
            value: <CopyableId value={SHA256} label={t`Скопировать SHA-256`} />,
          },
          { label: t`Целевая стадия`, value: "release" },
        ]}
      />
      <Section title={t`История`}>
        <Timeline label={t`История версии 1.4.2`} items={lifecycle()} />
      </Section>
      <Section
        title={t`Изменения манифеста`}
        aside={
          <ActionButton
            action={{
              label: t`Сравнить с 1.4.1`,
              run: { href: "/packages/acme-ui/compare/1.4.1...1.4.2" },
            }}
            variant="ghost"
          />
        }
      >
        <DiffViewer label={t`Изменения манифеста 1.4.1 → 1.4.2`} rows={DIFF_ROWS} />
      </Section>
    </div>
  );
}

function FilesTab() {
  const { t } = useLingui();
  return (
    <Table aria-label={t`Файлы версии 1.4.2`}>
      <TableHeader>
        <TableRow>
          <TableHead>{t`Путь`}</TableHead>
          <TableHead className="text-right">{t`Размер`}</TableHead>
          <TableHead>{"SHA-256"}</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        {FILES.map(([path, size], index) => (
          <TableRow key={path}>
            <TableCell className="font-mono text-caption">{path}</TableCell>
            <TableCell className="text-right tabular-nums">{activeFormat().bytes(size)}</TableCell>
            <TableCell>
              <CopyableId
                value={`${SHA256.slice(index * 3)}${SHA256.slice(0, index * 3)}`}
                label={t`Скопировать SHA-256 файла ${path}`}
              />
            </TableCell>
          </TableRow>
        ))}
      </TableBody>
    </Table>
  );
}

function panelTabs() {
  return [
    ["summary", i18n._(msg`Сводка`)],
    ["files", i18n._(msg`Файлы`)],
    ["history", i18n._(msg`История`)],
    ["metadata", i18n._(msg`Метаданные`)],
  ] as const;
}

type PanelTab = ReturnType<typeof panelTabs>[number][0];

function PanelContent({ tab }: { tab: PanelTab }) {
  const { t } = useLingui();
  if (tab === "files") {
    return <FilesTab />;
  }
  if (tab === "history") {
    return <Timeline label={t`История версии 1.4.2`} items={lifecycle()} />;
  }
  if (tab === "metadata") {
    return (
      <div className="flex flex-col gap-6">
        <JsonViewer label={t`Метаданные версии`} value={METADATA} />
        <Section title={t`Изменения манифеста`}>
          <DiffViewer label={t`Изменения манифеста 1.4.1 → 1.4.2`} rows={DIFF_ROWS} />
        </Section>
      </div>
    );
  }
  return <SummaryTab />;
}

export function VersionPanel({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const { t } = useLingui();
  const [tab, setTab] = useState<PanelTab>("summary");
  const changeTab = useCallback((value: unknown) => setTab(value as PanelTab), []);
  return (
    <DetailPanel
      open={open}
      onOpenChange={onOpenChange}
      title={t`Версия 1.4.2`}
      status={<VersionStatus version={{ state: "quarantined", block_reason: null }} />}
      menu={[
        {
          label: t`Открыть полностью`,
          run: { href: "/packages/acme-ui/versions/1.4.2" },
        },
      ]}
      tabs={
        <Tabs value={tab} onValueChange={changeTab}>
          <TabsList variant="line" aria-label={t`Разделы версии`}>
            {panelTabs().map(([value, label]) => (
              <TabsTrigger key={value} value={value}>
                {label}
              </TabsTrigger>
            ))}
          </TabsList>
        </Tabs>
      }
      footer={{
        primary: { label: t`Запросить одобрение`, run: { onSelect: noop } },
        secondary: [{ label: "Yank", run: { onSelect: noop } }],
        menu: [
          { label: t`Скачать SBOM и evidence`, run: { onSelect: noop } },
          {
            label: t`Удалить…`,
            run: { onSelect: noop },
            destructive: true,
            stepUp: true,
          },
        ],
      }}
    >
      <PanelContent tab={tab} />
    </DetailPanel>
  );
}
