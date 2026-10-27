// i18n: не мигрирован
/** Общие данные экранов: пользователи, workspace'ы и репозитории CRANE. */
export const CURRENT_USER = {
  name: "Иванов И.",
  initials: "ИИ",
  role: "владелец библиотеки",
} as const;

export const WORKSPACES = ["platform", "payments", "mobile", "data"] as const;

export const REPOSITORIES = [
  { id: "npm-hosted", format: "npm", kind: "hosted" },
  { id: "npm-proxy", format: "npm", kind: "proxy" },
  { id: "pypi-hosted", format: "PyPI", kind: "hosted" },
  { id: "maven-central-proxy", format: "Maven", kind: "proxy" },
  { id: "nuget-hosted", format: "NuGet", kind: "hosted" },
  { id: "cargo-sparse", format: "Cargo", kind: "proxy" },
] as const;

export const PEOPLE = [
  "Иванов И.",
  "Петрова А.",
  "Смирнов Д.",
  "Кузнецова Е.",
  "Орлов М.",
  "Белова Н.",
] as const;
