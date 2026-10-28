const FALLBACK = "/";
const BASE = "http://crane.invalid";
const LAST_CONTROL = 0x1f;
const DELETE = 0x7f;
const LOGIN_OR_SETUP = /^\/(login|setup)(\/|$)/;

function isControl(char: string): boolean {
  const code = char.codePointAt(0) ?? 0;
  return code <= LAST_CONTROL || code === DELETE;
}

function decoded(path: string): string {
  try {
    return decodeURIComponent(path).toLowerCase();
  } catch {
    // битая последовательность %: сравниваем как есть, она не может скрыть /login
    return path.toLowerCase();
  }
}

/**
 * Куда вернуть пользователя после входа. Принимается только путь того же источника: начинается с одной «/»,
 * не «//», не ведёт на /login или /setup. Всё остальное (чужой хост, схема, управляющие символы, обратная косая)
 * даёт «/»: иначе параметр `redirect` стал бы открытым редиректом.
 */
export function sanitizeRedirect(value: unknown): string {
  if (typeof value !== "string" || !value.startsWith("/") || value.startsWith("//")) {
    return FALLBACK;
  }
  // браузер читает «/\evil.example» и «/<таб>/evil.example» как «//evil.example»
  if (value.includes("\\") || Array.from(value).some(isControl)) {
    return FALLBACK;
  }
  const url = new URL(value, BASE);
  if (LOGIN_OR_SETUP.test(decoded(url.pathname))) {
    return FALLBACK;
  }
  return value;
}
