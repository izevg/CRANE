import { useLingui } from "@lingui/react/macro";
import { useQuery } from "@tanstack/react-query";
import { Outlet, useMatchRoute, useRouter } from "@tanstack/react-router";
import { DEMO, getGetMeQueryOptions } from "~/shared/api";
import { AppShell, type Banner } from "~/widgets/app-shell";
import { useSessionEnded } from "../../query/useSessionEnded";
import { DemoNotice } from "../demo/DemoNotice";
import { usePageTitle } from "../focus/useDocumentTitle";
import { navGroups } from "../nav/navItems";

const HELP_HREF = "/help";
const LOGIN_PATH = "/login";

/**
 * Шелл приложения. Навигация строится из `_links` ответа `GET /me` (решение D6) и показывает только пункты, для
 * которых в роутере есть маршрут. Плашка демо стоит над содержимым страницы, внутри `<main>`.
 */
export function ShellLayout() {
  const { t } = useLingui();
  const router = useRouter();
  const matchRoute = useMatchRoute();
  const sessionEnded = useSessionEnded();
  const page = usePageTitle();
  // Данные положил в кэш beforeLoad шелла: отдельного запроса здесь нет.
  const { data } = useQuery(getGetMeQueryOptions());
  const loginHref = `${LOGIN_PATH}?redirect=${encodeURIComponent(router.latestLocation.href)}`;
  const sessionEndedBanner: Banner = {
    id: "session-ended",
    tone: "warning",
    text: t`Сессия завершена`,
    action: { label: t`Войти снова`, run: { href: loginHref } },
  };
  const groups = data
    ? navGroups(data.data._links, {
        hasRoute: (to) => Object.hasOwn(router.routesByPath, to),
        isActive: (to) => matchRoute({ to, fuzzy: true }) !== false,
      })
    : [];
  return (
    <AppShell
      sidebar={{ groups, footerItems: [], helpHref: HELP_HREF }}
      topbar={{
        breadcrumbs: { trail: [], current: page ?? "" },
        onOpenPalette: () => undefined,
      }}
      banners={sessionEnded ? [sessionEndedBanner] : []}
    >
      {DEMO ? <DemoNotice /> : null}
      <Outlet />
    </AppShell>
  );
}
