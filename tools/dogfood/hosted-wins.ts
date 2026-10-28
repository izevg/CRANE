// Hosted побеждает proxy на настоящем стенде: имя, опубликованное в hosted, proxy не отдаёт (404)
// и в upstream ради него не ходит.
import { type Expected, egressLines } from "./audit.ts";
import { authorizedCurl } from "./cargo.ts";
import type { Client } from "./client.ts";
import type { Stand } from "./stand.ts";

const HTTP_NOT_FOUND = 404;
/** Схема заголовка авторизации npm: слой аутентификации ждёт `Bearer <токен>`, а не голый токен cargo. */
const NPM_SCHEME = "Bearer";
const FIRST_SEGMENT_END = 2;
const SECOND_SEGMENT_END = 4;
const SHORT_NAME_MAX = 2;
const THREE_CHARS = 3;

/** Раскладка sparse-индекса (crates/formats/cargo/src/index/mod.rs::index_path); имя в нижнем регистре. */
export function indexPath(name: string): string {
  const lower = name.toLowerCase();
  if (lower.length <= SHORT_NAME_MAX) {
    return `${lower.length}/${lower}`;
  }
  if (lower.length === THREE_CHARS) {
    return `3/${lower.slice(0, 1)}/${lower}`;
  }
  return `${lower.slice(0, FIRST_SEGMENT_END)}/${lower.slice(FIRST_SEGMENT_END, SECOND_SEGMENT_END)}/${lower}`;
}

/** User-Agent curl из `authorizedCurl`: по нему аудит отличает отказ proxy от событий клиента пакетов. */
export const CURL_AGENT = "curl/";

/**
 * Запрашивает у proxy документ имени (`url`) и проверяет 404. Выборки из upstream, чей путь содержит
 * `needle`, считаются до и после: публикация уже спросила публичный реестр о новом имени (одна выборка),
 * запрос к proxy не должен добавить вторую. `scheme` — схема заголовка авторизации формата (npm: `Bearer`,
 * cargo: без схемы); параметр обязательный, чтобы запрос без верных учётных данных (403 от слоя
 * аутентификации вместо 404 от защиты имён) не получался по умолчанию. Возвращает событие аудита отказа.
 */
export function expectHostedWins(
  stand: Stand,
  client: Client,
  url: string,
  needle: string,
  scheme: string | undefined,
): Expected {
  const fetches = (): number =>
    egressLines(stand.craneLogs()).filter((line) => line.path.includes(needle)).length;
  const before = fetches();
  const status = authorizedCurl(
    client,
    stand.token,
    ["-sS", "-o", "/dev/null", "-w", "%{http_code}", url],
    scheme,
  );
  if (status.stdout.trim() !== String(HTTP_NOT_FOUND)) {
    throw new Error(
      `hosted-имя ${needle} через proxy: ожидался 404, получен ${status.stdout.trim()}`,
    );
  }
  if (fetches() !== before) {
    throw new Error(`hosted-имя ${needle}: proxy сходил в upstream`);
  }
  return { kind: "shadow", agent: CURL_AGENT };
}

/**
 * Для npm: новое имя зарезервировано первой публикацией, а его packument через proxy (`base` с завершающим
 * слэшем) отдаёт 404. Возвращает оба события аудита: резервирование и отказ proxy.
 */
export function reservedAndHidden(
  stand: Stand,
  client: Client,
  agent: string,
  base: string,
  name: string,
): Expected[] {
  const url = `${base}${name.replace("/", "%2f")}`;
  const needle = name.split("/").pop() ?? name;
  return [{ kind: "reserve", agent }, expectHostedWins(stand, client, url, needle, NPM_SCHEME)];
}
