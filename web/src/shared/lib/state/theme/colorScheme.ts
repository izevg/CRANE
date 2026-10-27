/**
 * Цветовая схема (light / dark / system). Порт из UI kit BRICK.
 *
 * Выбор пользователя хранится в localStorage под ключом `crane.colorScheme`. На `<html data-theme>` всегда лежит
 * РАЗРЕШЁННОЕ значение (light | dark): для `system` оно вычисляется из `prefers-color-scheme` и следует за ним.
 * Начальное значение до загрузки бандла ставит `public/theme-init.js`, `initColorScheme()` подхватывает его в React.
 * Любой сбой storage или matchMedia деградирует в `system` / light без исключений.
 */

export const COLOR_SCHEME_STORAGE_KEY = "crane.colorScheme";

const COLOR_MODES = ["light", "dark", "system"] as const;
export type ColorMode = (typeof COLOR_MODES)[number];
export type ResolvedScheme = "light" | "dark";

const DARK_QUERY = "(prefers-color-scheme: dark)";

function isColorMode(value: unknown): value is ColorMode {
  return typeof value === "string" && (COLOR_MODES as readonly string[]).includes(value);
}

function safeStorage(): Storage | undefined {
  try {
    return typeof window !== "undefined" ? window.localStorage : undefined;
  } catch {
    return undefined;
  }
}

export function readStoredMode(storage: Storage | undefined = safeStorage()): ColorMode {
  try {
    const value = storage?.getItem(COLOR_SCHEME_STORAGE_KEY);
    return isColorMode(value) ? value : "system";
  } catch {
    return "system";
  }
}

export function writeStoredMode(
  mode: ColorMode,
  storage: Storage | undefined = safeStorage(),
): boolean {
  try {
    if (storage === undefined) {
      return false;
    }
    storage.setItem(COLOR_SCHEME_STORAGE_KEY, mode);
    return true;
  } catch {
    return false;
  }
}

function systemQuery(): MediaQueryList | undefined {
  try {
    return typeof window !== "undefined" && typeof window.matchMedia === "function"
      ? window.matchMedia(DARK_QUERY)
      : undefined;
  } catch {
    return undefined;
  }
}

function systemPrefersDark(): boolean {
  return systemQuery()?.matches === true;
}

function resolveScheme(mode: ColorMode, prefersDark: boolean): ResolvedScheme {
  return mode === "system" ? (prefersDark ? "dark" : "light") : mode;
}

/** Ставит `data-theme` и `color-scheme` (нативные контролы, скроллбары) на корневой элемент. */
function applyScheme(scheme: ResolvedScheme, root: HTMLElement = document.documentElement): void {
  root.setAttribute("data-theme", scheme);
  root.style.colorScheme = scheme;
}

const NO_TRANSITIONS_CSS = "*,*::before,*::after{transition:none!important}";

/**
 * Смена темы мгновенная: на один кадр отключаем CSS-транзиции, форсируем пересчёт стилей и снимаем блокировку.
 * Так ни одна транзиция цвета не запускается (в `document.getAnimations()` пусто).
 */
function withoutTransitions(change: () => void): void {
  if (typeof document === "undefined") {
    change();
    return;
  }
  const style = document.createElement("style");
  style.textContent = NO_TRANSITIONS_CSS;
  document.head.appendChild(style);
  try {
    change();
    void getComputedStyle(document.body).opacity;
  } finally {
    const release = () => style.remove();
    if (typeof requestAnimationFrame === "function") {
      requestAnimationFrame(release);
    } else {
      setTimeout(release, 0);
    }
  }
}

let currentMode: ColorMode = "system";
const listeners = new Set<() => void>();

function notify(): void {
  for (const listener of listeners) {
    listener();
  }
}

function resolveCurrent(): ResolvedScheme {
  return resolveScheme(currentMode, systemPrefersDark());
}

function applyCurrent(): void {
  const scheme = resolveCurrent();
  withoutTransitions(() => applyScheme(scheme));
}

export function getColorMode(): ColorMode {
  return currentMode;
}

/** Разрешённая схема для текущего режима (для компонентов, которым нужен light|dark, например Toaster). */
export function getResolvedScheme(): ResolvedScheme {
  return resolveCurrent();
}

export function subscribeColorMode(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

export function setColorMode(mode: ColorMode): void {
  currentMode = mode;
  writeStoredMode(mode);
  applyCurrent();
  notify();
}

let teardown: (() => void) | undefined;

/**
 * Читает сохранённый режим, применяет его к <html> и подписывается на смену системной темы и на изменения из
 * другой вкладки. Идемпотентна: повторный вызов заменяет предыдущую подписку. Возвращает функцию очистки.
 */
export function initColorScheme(): () => void {
  teardown?.();
  currentMode = readStoredMode();
  applyScheme(resolveCurrent());
  notify();

  const query = systemQuery();
  const onSystemChange = () => {
    if (currentMode === "system") {
      applyCurrent();
      notify();
    }
  };
  const onStorage = (event: StorageEvent) => {
    if (event.key !== null && event.key !== COLOR_SCHEME_STORAGE_KEY) {
      return;
    }
    const next = readStoredMode();
    if (next !== currentMode) {
      currentMode = next;
      applyCurrent();
      notify();
    }
  };

  query?.addEventListener?.("change", onSystemChange);
  window.addEventListener("storage", onStorage);

  const cleanup = () => {
    query?.removeEventListener?.("change", onSystemChange);
    window.removeEventListener("storage", onStorage);
    if (teardown === cleanup) {
      teardown = undefined;
    }
  };
  teardown = cleanup;
  return cleanup;
}
