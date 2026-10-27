/** Какое хранилище браузера: `local` живёт между сессиями, `session` — до закрытия вкладки. */
export type StorageKind = "local" | "session";

function storageOf(kind: StorageKind): Storage | undefined {
  try {
    if (typeof window === "undefined") {
      return undefined;
    }
    return kind === "local" ? window.localStorage : window.sessionStorage;
  } catch {
    // доступ к storage сам может бросить (запрещённые куки, песочница)
    return undefined;
  }
}

/**
 * Читает строку и проверяет её `isValid`. Любой сбой (нет storage, исключение, невалидное значение) даёт
 * `undefined`: вызывающий подставляет значение по умолчанию. Кодировка не знает про JSON: хранятся только строки,
 * структуры сериализует вызывающий.
 */
export function readStored<T extends string>(
  kind: StorageKind,
  key: string,
  isValid: (value: string) => value is T,
): T | undefined {
  try {
    const raw = storageOf(kind)?.getItem(key);
    return raw !== null && raw !== undefined && isValid(raw) ? raw : undefined;
  } catch {
    return undefined;
  }
}

/** Пишет строку; `false`, если сохранить не удалось (выбор тогда живёт до перезагрузки). */
export function writeStored(kind: StorageKind, key: string, value: string): boolean {
  try {
    const storage = storageOf(kind);
    if (storage === undefined) {
      return false;
    }
    storage.setItem(key, value);
    return true;
  } catch {
    return false;
  }
}

export function removeStored(kind: StorageKind, key: string): void {
  try {
    storageOf(kind)?.removeItem(key);
  } catch {
    // нечего удалять: хранилище недоступно
  }
}
