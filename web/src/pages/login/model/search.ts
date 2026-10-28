import type { SessionReason } from "~/shared/ui/status";
import { sanitizeRedirect } from "./redirect";

export type LoginSearch = {
  /** Причина завершения сессии из 401; без неё это первый вход. */
  reason?: SessionReason;
  /** Куда вернуть после входа: только путь того же источника. */
  redirect?: string;
};

const REASONS: readonly unknown[] = ["session_expired", "session_revoked", "session_elsewhere"];

function isReason(value: unknown): value is SessionReason {
  return REASONS.includes(value);
}

/** Параметры /login из URL. Всё лишнее и неверное отбрасывается, `redirect` проходит санитизацию. */
export function parseLoginSearch(search: Record<string, unknown>): LoginSearch {
  const { reason, redirect } = search;
  return {
    ...(isReason(reason) ? { reason } : {}),
    ...(redirect === undefined ? {} : { redirect: sanitizeRedirect(redirect) }),
  };
}
