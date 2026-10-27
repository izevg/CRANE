/** Какое хранилище браузера: `local` живёт между сессиями, `session` — до закрытия вкладки. */
type StorageKind = "local" | "session";
/** Хранилище задаётся видом или готовым объектом `Storage` (подмена в тестах). */
export type StorageSource = StorageKind | Storage;

function storageOf(source: StorageSource): Storage | undefined {
  if (typeof source !== "string") {
    return source;
  }
  try {
    if (typeof window === "undefined") {
      return undefined;
    }
    return source === "local" ? window.localStorage : window.sessionStorage;
  } catch {
    // доступ к storage сам может бросить (запрещённые куки, песочница)
    return undefined;
  }
}

/** Строка хранится как есть (совместимо с ключами, записанными до хелпера), остальное — как JSON. */
function serialize(value: unknown): string {
  return typeof value === "string" ? value : JSON.stringify(value);
}

/**
 * Что считать значением записи: JSON-объект и массив — только разобранными; примитив — разобранным, а если он
 * не подошёл, то и строкой как есть (ключи, записанные до хелпера, лежат строками без JSON).
 */
function candidates(raw: string): unknown[] {
  try {
    const parsed = JSON.parse(raw) as unknown;
    return typeof parsed === "object" && parsed !== null ? [parsed] : [parsed, raw];
  } catch {
    return [raw];
  }
}

/**
 * Читает значение и проверяет его `isValid`. Любой сбой (нет storage, исключение, чужой формат, невалидное
 * значение) даёт `undefined`: вызывающий подставляет значение по умолчанию.
 */
export function readStored<T>(
  source: StorageSource,
  key: string,
  isValid: (value: unknown) => value is T,
): T | undefined {
  try {
    const raw = storageOf(source)?.getItem(key);
    if (raw === null || raw === undefined) {
      return undefined;
    }
    return candidates(raw).find(isValid);
  } catch {
    return undefined;
  }
}

/** Пишет значение; `false`, если сохранить не удалось (выбор тогда живёт до перезагрузки). */
export function writeStored(source: StorageSource, key: string, value: unknown): boolean {
  try {
    const storage = storageOf(source);
    if (storage === undefined) {
      return false;
    }
    storage.setItem(key, serialize(value));
    return true;
  } catch {
    return false;
  }
}

export function removeStored(source: StorageSource, key: string): void {
  try {
    storageOf(source)?.removeItem(key);
  } catch {
    // нечего удалять: хранилище недоступно
  }
}
