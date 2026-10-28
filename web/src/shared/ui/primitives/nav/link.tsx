import type * as React from "react";
import { type ComponentType, createContext, type ReactNode, useContext } from "react";

export type LinkProps = Omit<React.ComponentProps<"a">, "href"> & { href: string };

type LinkComponent = ComponentType<LinkProps>;

function AnchorLink(props: LinkProps) {
  return <a {...props} />;
}

const LinkContext = createContext<LinkComponent>(AnchorLink);

/**
 * Шов роутера: кит не знает о роутере, приложение один раз подставляет его компонент ссылки.
 * Без провайдера ссылки — обычные `<a href>` (полная перезагрузка страницы).
 */
export function LinkProvider({
  component,
  children,
}: {
  component: LinkComponent;
  children: ReactNode;
}) {
  return <LinkContext.Provider value={component}>{children}</LinkContext.Provider>;
}

/** Все ссылки кита идут через этот компонент. */
export function Link(props: LinkProps) {
  const Component = useContext(LinkContext);
  return <Component {...props} />;
}
