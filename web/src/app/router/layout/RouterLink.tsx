import { Link } from "@tanstack/react-router";
import { hrefKind } from "~/shared/lib/route";
import type { LinkProps } from "~/shared/ui/primitives";

/** Пропсы якоря с явным `undefined` не ложатся на типы `Link` при `exactOptionalPropertyTypes`: такие ключи убираем. */
type Defined<T> = { [K in keyof T]: Exclude<T[K], undefined> };

function defined<T extends object>(props: T): Defined<T> {
  return Object.fromEntries(
    Object.entries(props).filter(([, value]) => value !== undefined),
  ) as Defined<T>;
}

const EXTERNAL_REL = "noopener noreferrer nofollow";
const MAILTO_REL = "noopener noreferrer";

/**
 * Ссылка кита поверх роутера: `Link` с опцией `href` разбирает путь вместе с query и hash, ставит `aria-current`,
 * подгружает страницу по наведению и фокусу и не перехватывает клики с модификаторами. Адрес без маршрута тоже
 * переходит внутри SPA и покажет 404 шелла. Чужой адрес (`http(s):`, `//`, `mailto:`) — обычный `<a>`; адрес с любой
 * другой схемой (`javascript:`, `data:`) не ссылка, а текст: href приходит из метаданных пакетов, то есть от чужих людей.
 */
export function RouterLink({ href, ...rest }: LinkProps) {
  const kind = hrefKind(href);
  if (kind === "blocked") {
    return <span className={rest.className}>{rest.children}</span>;
  }
  if (kind === "internal") {
    // `to` обязателен по типам; при заданном `href` роутер разбирает адрес из него
    return <Link to="." href={href} {...defined(rest)} />;
  }
  return <a href={href} {...rest} rel={kind === "external" ? EXTERNAL_REL : MAILTO_REL} />;
}
