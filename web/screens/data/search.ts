import type { VersionState } from "~/shared/api";
import type { Tone } from "~/shared/ui/primitives";

export type SearchResult = {
  id: string;
  name: string;
  format: string;
  repository: string;
  workspace: string;
  description: string;
  version: string;
  state: Extract<VersionState, "released" | "quarantined" | "blocked">;
  downloads: number;
  team: string;
  updated: string;
  exact?: true;
  deprecated?: true;
  typosquatOf?: string;
  risk?: { priority: "P0" | "P1" | "P2" | "P3"; findings: number };
};

const RISK_TONE = { P0: "error", P1: "error", P2: "warning", P3: "info" } as const satisfies Record<
  string,
  Tone
>;

export function riskTone(priority: keyof typeof RISK_TONE): Tone {
  return RISK_TONE[priority];
}

const RELEASED = "released";
const QUARANTINE = "quarantined";
const BLOCKED = "blocked";

// i18n-fixture: результаты поиска (названия, описания, команды) — контент витрины, а не текст интерфейса
export const RESULTS: readonly SearchResult[] = [
  {
    id: "acme-ui",
    name: "@acme/ui",
    format: "npm",
    repository: "npm-hosted",
    workspace: "platform",
    description: "Дизайн-система Acme: React-компоненты, токены и темы для внутренних приложений",
    version: "1.4.2",
    state: RELEASED,
    downloads: 12340,
    team: "Команда UI-kit",
    updated: "2026-10-24T09:15:00+03:00",
    exact: true,
    risk: { priority: "P1", findings: 1 },
  },
  {
    id: "acme-uii",
    name: "@acme/uii",
    format: "npm",
    repository: "npm-proxy",
    workspace: "platform",
    description: "UI components for acme apps (fast and lightweight)",
    version: "0.0.1",
    state: QUARANTINE,
    downloads: 37,
    team: "Не назначена",
    updated: "2026-10-26T22:41:00+03:00",
    typosquatOf: "@acme/ui",
    risk: { priority: "P0", findings: 2 },
  },
  {
    id: "acme-ui-icons",
    name: "@acme/ui-icons",
    format: "npm",
    repository: "npm-hosted",
    workspace: "platform",
    description: "Набор из 1 280 SVG-иконок с поддержкой tree-shaking и тёмной темы",
    version: "3.2.0",
    state: RELEASED,
    downloads: 9815,
    team: "Команда UI-kit",
    updated: "2026-10-21T14:02:00+03:00",
  },
  {
    id: "acme-ui-legacy",
    name: "@acme/ui-legacy-bootstrap-adapter",
    format: "npm",
    repository: "npm-hosted",
    workspace: "payments",
    description: "Адаптер Bootstrap 3 для старых экранов биллинга; заменён на @acme/ui",
    version: "0.9.14",
    state: RELEASED,
    downloads: 412,
    team: "Команда платежей",
    updated: "2026-05-12T11:30:00+03:00",
    deprecated: true,
    risk: { priority: "P2", findings: 4 },
  },
  {
    id: "acme-ui-react",
    name: "acme-ui-react-native-bridge",
    format: "npm",
    repository: "npm-proxy",
    workspace: "mobile",
    description: "Мост между @acme/ui и React Native: общие токены, шрифты и spacing-шкала",
    version: "2.0.0-rc.3",
    state: QUARANTINE,
    downloads: 1204,
    team: "Команда мобильной разработки",
    updated: "2026-10-26T16:20:00+03:00",
  },
  {
    id: "acme-ui-py",
    name: "acme-ui-tokens",
    format: "PyPI",
    repository: "pypi-hosted",
    workspace: "data",
    description: "Дизайн-токены Acme для отчётов Jupyter и дашбордов matplotlib",
    version: "0.7.3",
    state: RELEASED,
    downloads: 2761,
    team: "Команда данных",
    updated: "2026-10-19T08:45:00+03:00",
    risk: { priority: "P3", findings: 1 },
  },
  {
    id: "acme-ui-render",
    name: "acme-ui-report-renderer",
    format: "PyPI",
    repository: "pypi-hosted",
    workspace: "data",
    description: "Генерация PDF-отчётов в стиле Acme на WeasyPrint и Jinja2",
    version: "1.1.0",
    state: RELEASED,
    downloads: 886,
    team: "Команда данных",
    updated: "2026-10-02T17:10:00+03:00",
  },
  {
    id: "acme-ui-bom",
    name: "com.acme.platform:acme-ui-bom",
    format: "Maven",
    repository: "maven-central-proxy",
    workspace: "platform",
    description: "BOM для согласованных версий серверных компонентов Acme UI Gateway",
    version: "5.3.1",
    state: RELEASED,
    downloads: 3092,
    team: "Команда платформы",
    updated: "2026-10-23T12:00:00+03:00",
    risk: { priority: "P1", findings: 3 },
  },
  {
    id: "acme-ui-spring",
    name: "com.acme.payments:ui-session-spring-starter",
    format: "Maven",
    repository: "maven-central-proxy",
    workspace: "payments",
    description: "Spring Boot starter для UI-сессий; отозван из-за CVE-2026-31337",
    version: "2.8.4",
    state: BLOCKED,
    downloads: 1570,
    team: "Команда платежей",
    updated: "2026-10-25T03:12:00+03:00",
    risk: { priority: "P0", findings: 1 },
  },
  {
    id: "acme-ui-blazor",
    name: "Acme.Ui.Blazor.Components",
    format: "NuGet",
    repository: "nuget-hosted",
    workspace: "platform",
    description: "Компоненты Blazor для внутренних порталов: таблицы, формы, диалоги",
    version: "4.0.2",
    state: RELEASED,
    downloads: 654,
    team: "Команда .NET",
    updated: "2026-10-15T10:25:00+03:00",
  },
  {
    id: "acme-ui-tokens-rs",
    name: "acme-ui-tokens-sys",
    format: "Cargo",
    repository: "cargo-sparse",
    workspace: "mobile",
    description: "Rust-биндинги к токенам дизайн-системы для встраиваемых экранов",
    version: "0.3.0",
    state: RELEASED,
    downloads: 118,
    team: "Команда мобильной разработки",
    updated: "2026-09-30T19:05:00+03:00",
  },
  {
    id: "acme-ui-theme",
    name: "@acme-internal/ui-theme-contrast-checker",
    format: "npm",
    repository: "npm-hosted",
    workspace: "platform",
    description: "CLI проверки контраста тем по WCAG 2.2: правила, отчёты, интеграция с CI",
    version: "1.0.7",
    state: RELEASED,
    downloads: 2210,
    team: "Команда UI-kit",
    updated: "2026-10-27T08:40:00+03:00",
    risk: { priority: "P2", findings: 2 },
  },
];

export const FORMAT_FACETS = [
  { value: "npm", label: "npm", count: { count: 21, capped: false } },
  { value: "pypi", label: "PyPI", count: { count: 8, capped: false } },
  { value: "maven", label: "Maven", count: { count: 5, capped: false } },
  { value: "nuget", label: "NuGet", count: { count: 3, capped: false } },
  { value: "cargo", label: "Cargo", count: { count: 1, capped: false } },
] as const;

export const WORKSPACE_FACETS = [
  { value: "platform", label: "platform", count: { count: 19, capped: false } },
  { value: "payments", label: "payments", count: { count: 9, capped: false } },
  { value: "mobile", label: "mobile", count: { count: 6, capped: false } },
  { value: "data", label: "data", count: { count: 4, capped: false } },
] as const;
