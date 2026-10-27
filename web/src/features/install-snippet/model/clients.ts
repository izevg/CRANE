import type { Format, InstallClient } from "~/shared/api";

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

/** Значение из хранилища — известный клиент (список для формата приходит с сервера: `available_clients`). */
export function isInstallClient(value: unknown): value is InstallClient {
  return typeof value === "string" && Object.hasOwn(CLIENT_LABEL, value);
}

/** Ключ хранилища: выбор клиента помнится отдельно для каждого формата. */
export function storageKey(format: Format): string {
  return `crane.install.${format}`;
}
