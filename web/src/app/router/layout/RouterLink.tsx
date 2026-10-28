import { Link } from "@tanstack/react-router";
import type { LinkProps } from "~/shared/ui/primitives";

/** Пропсы якоря с явным `undefined` не ложатся на типы `Link` при `exactOptionalPropertyTypes`: такие ключи убираем. */
type Defined<T> = { [K in keyof T]: Exclude<T[K], undefined> };

function defined<T extends object>(props: T): Defined<T> {
  return Object.fromEntries(
    Object.entries(props).filter(([, value]) => value !== undefined),
  ) as Defined<T>;
}

const SCHEME = /^[a-z][a-z\d+.-]*:/i;

/**
 * Ссылка кита поверх роутера: `Link` с опцией `href` разбирает путь вместе с query и hash, ставит `aria-current`,
 * подгружает страницу по наведению и фокусу и не перехватывает клики с модификаторами. Адрес без маршрута тоже
 * переходит внутри SPA и покажет 404 шелла. Обычный `<a>` — только адрес со схемой или `//`.
 */
export function RouterLink({ href, ...rest }: LinkProps) {
  return SCHEME.test(href) || href.startsWith("//") ? (
    <a href={href} {...rest} />
  ) : (
    // `to` обязателен по типам; при заданном `href` роутер разбирает адрес из него
    <Link to="." href={href} {...defined(rest)} />
  );
}
