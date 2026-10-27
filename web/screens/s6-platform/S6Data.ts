// biome-ignore-all lint/style/noMagicNumbers: детерминированные данные макета, не продуктовый код
import { msg } from "@lingui/core/macro";
import { i18n } from "~/shared/lib/i18n";
import { activeFormat } from "../frame/format";

export const STALE_TEXT = /обновлено/;
export const TB_EN = /\d\.\d TB/;
export const TB = 1024 ** 4;

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

export type Dimension = "team" | "format";
export function dimensions() {
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

export const SPARK_VOLUME = [16.9, 17.1, 17.0, 17.4, 17.6, 17.9, 18.0, 18.2, 18.4];
export const SPARK_HIT = [93.1, 93.8, 94.4, 93.9, 94.6, 95.0, 94.2, 94.8, 94.2];

// 24 ч по 5 минут: детерминированный шум, чтобы снимок не плавал между прогонами.
const STEP_MS = 5 * 60 * 1000;
const POINTS = 288;
const END = Date.UTC(2026, 9, 27, 11, 0, 0);
export const SLO_MS = 150;
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

export const P50 = latency(28, 4, 0.3, 4);
export const P95 = latency(88, 12, 1.1, 22);
export const P99 = latency(128, 14, 2.4, 34);
const OVER_SLO = P99.filter((point) => point.v > SLO_MS).length;
export const formatBytes = (bytes: number): string => activeFormat().bytes(bytes);

const timeOf = (ts: number): string =>
  new Intl.DateTimeFormat(i18n.locale, { hour: "2-digit", minute: "2-digit" }).format(ts);
const P95_PEAK = P95.reduce((best, point) => (point.v > best.v ? point : best));
export const summary = () => {
  const average = activeFormat().number(Math.round(P95.reduce((sum, p) => sum + p.v, 0) / POINTS));
  const peak = activeFormat().number(P95_PEAK.v);
  const peakTime = timeOf(P95_PEAK.t);
  const overMinutes = activeFormat().number(OVER_SLO * 5);
  const totalMinutes = activeFormat().number(POINTS * 5);
  return i18n._(
    msg`Задержки за 24 ч, шаг 5 мин. p95: в среднем ${average} мс, максимум ${peak} мс в ${peakTime}. p99 выше SLO ${SLO_MS} мс ${overMinutes} мин из ${totalMinutes}.`,
  );
};
