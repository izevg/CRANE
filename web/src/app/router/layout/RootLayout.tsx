import { Outlet } from "@tanstack/react-router";
import { LinkProvider } from "~/shared/ui/primitives";
import { useDocumentTitle } from "../focus/useDocumentTitle";
import { useRouteFocus } from "../focus/useRouteFocus";
import { RouterLink } from "./RouterLink";

/** Корень дерева: ссылки кита через роутер, фокус на h1 при смене страницы и document.title. */
export function RootLayout() {
  useRouteFocus();
  useDocumentTitle();
  return (
    <LinkProvider component={RouterLink}>
      <Outlet />
    </LinkProvider>
  );
}
