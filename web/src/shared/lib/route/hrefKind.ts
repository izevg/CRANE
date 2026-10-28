export type HrefKind = "internal" | "external" | "mailto" | "blocked";

const SCHEME = /^([a-z][a-z\d+.-]*):/i;
const WEB_SCHEMES = new Set(["http", "https"]);
const MAILTO = "mailto";
const TAB = 0x09;
const LINE_FEED = 0x0a;
const CARRIAGE_RETURN = 0x0d;
const SPACE = 0x20;
/** Табуляцию и переводы строк браузер выбрасывает из адреса целиком, а управляющие символы и пробелы - в его начале. */
const URL_TAB_OR_NEWLINE = new Set([TAB, LINE_FEED, CARRIAGE_RETURN]);
const LAST_LEADING_STRIPPED = SPACE;

function isSlash(char: string | undefined): boolean {
  // для http(s) браузер читает обратную косую как прямую: `/\evil.example` — это `//evil.example`
  return char === "/" || char === "\\";
}

/** Адрес так, как его увидит парсер URL браузера: ` javascript:` и `java\tscript:` остаются схемой. */
function normalize(href: string): string {
  const chars = [...href].filter((char) => !URL_TAB_OR_NEWLINE.has(char.codePointAt(0) ?? 0));
  const start = chars.findIndex((char) => (char.codePointAt(0) ?? 0) > LAST_LEADING_STRIPPED);
  return start === -1 ? "" : chars.slice(start).join("");
}

/**
 * Куда ведёт адрес ссылки. Внешним считается только `http:`/`https:` и адрес без схемы с `//`; `mailto:` — отдельно.
 * Любая другая схема (`javascript:`, `data:`, `vbscript:`, неизвестная) не ссылка: подставленный в `href` такой адрес
 * от автора пакета выполнил бы код в контексте консоли.
 */
export function hrefKind(href: string): HrefKind {
  const address = normalize(href);
  const scheme = SCHEME.exec(address)?.[1]?.toLowerCase();
  if (scheme !== undefined) {
    if (WEB_SCHEMES.has(scheme)) {
      return "external";
    }
    return scheme === MAILTO ? "mailto" : "blocked";
  }
  return isSlash(address[0]) && isSlash(address[1]) ? "external" : "internal";
}
