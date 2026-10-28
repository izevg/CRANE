import { useMatchRoute, useRouter } from "@tanstack/react-router";
import { type FocusEvent, type MouseEvent, useCallback } from "react";
import type { LinkProps } from "~/shared/ui/primitives";

const PRIMARY_BUTTON = 0;

function isPlainClick(event: MouseEvent<HTMLAnchorElement>, target: string | undefined): boolean {
  const modified = event.metaKey || event.ctrlKey || event.shiftKey || event.altKey;
  return (
    event.button === PRIMARY_BUTTON && !modified && (target === undefined || target === "_self")
  );
}

type AppPath = keyof ReturnType<typeof useRouter>["routesByPath"];

/** Ссылка на маршрут приложения: клиентский переход, подгрузка по наведению и фокусу, `aria-current` из совпадения. */
function AppLink({
  to,
  href: _href,
  onClick,
  onMouseEnter,
  onFocus,
  ...rest
}: LinkProps & { to: AppPath }) {
  const router = useRouter();
  const matchRoute = useMatchRoute();
  const preload = useCallback(() => {
    void router.preloadRoute({ to });
  }, [router, to]);
  const handleClick = useCallback(
    (event: MouseEvent<HTMLAnchorElement>) => {
      onClick?.(event);
      if (!event.defaultPrevented && isPlainClick(event, rest.target)) {
        event.preventDefault();
        void router.navigate({ to });
      }
    },
    [onClick, rest.target, router, to],
  );
  const handleHover = useCallback(
    (event: MouseEvent<HTMLAnchorElement>) => {
      onMouseEnter?.(event);
      preload();
    },
    [onMouseEnter, preload],
  );
  const handleFocus = useCallback(
    (event: FocusEvent<HTMLAnchorElement>) => {
      onFocus?.(event);
      preload();
    },
    [onFocus, preload],
  );
  return (
    <a
      aria-current={matchRoute({ to }) === false ? undefined : "page"}
      {...rest}
      href={router.buildLocation({ to }).href}
      onClick={handleClick}
      onMouseEnter={handleHover}
      onFocus={handleFocus}
    />
  );
}

/**
 * Ссылка кита поверх роутера: адрес с маршрутом в приложении — клиентский переход, остальное (адрес с query,
 * чужой, без маршрута) — обычный `<a>`. Свой `<a>`, а не `Link` роутера: кит передаёт произвольные пропсы якоря,
 * и их типы с `Link` не совпадают.
 */
export function RouterLink(props: LinkProps) {
  const router = useRouter();
  if (Object.hasOwn(router.routesByPath, props.href)) {
    return <AppLink {...props} to={props.href as AppPath} />;
  }
  return <a {...props} />;
}
