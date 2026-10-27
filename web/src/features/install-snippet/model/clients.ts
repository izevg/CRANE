import type { Format, InstallClient } from "~/shared/api";

/**
 * Клиенты, доступные для формата пакета: первый в списке — по умолчанию. Список живёт в одном модуле намеренно:
 * когда сервер начнёт отдавать `available_clients` (отдельный PR в спеку), замена сводится к правке этого файла,
 * а блок и хук не меняются.
 */
export const CLIENTS_BY_FORMAT = {
  npm: ["npm", "pnpm", "yarn"],
  pypi: ["pip", "uv", "poetry"],
  maven: ["maven", "gradle"],
  nuget: ["dotnet"],
  cargo: ["cargo"],
} as const satisfies Record<Format, readonly [InstallClient, ...InstallClient[]]>;

// i18n-fixture: названия инструментов не переводятся
const CLIENT_LABEL = {
  npm: "npm",
  pnpm: "pnpm",
  yarn: "yarn",
  pip: "pip",
  uv: "uv",
  poetry: "poetry",
  maven: "Maven",
  gradle: "Gradle",
  dotnet: "dotnet",
  cargo: "cargo",
} as const satisfies Record<InstallClient, string>;

export function clientLabel(client: InstallClient): string {
  return CLIENT_LABEL[client];
}

/** Ключ хранилища: выбор клиента помнится отдельно для каждого формата. */
export function storageKey(format: Format): string {
  return `crane.install.${format}`;
}
