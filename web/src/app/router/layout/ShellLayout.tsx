import { useLingui } from "@lingui/react/macro";
import { useQuery } from "@tanstack/react-query";
import { Outlet, useMatchRoute, useRouter } from "@tanstack/react-router";
import { DEMO, getGetMeQueryOptions, SIGN_IN_AVAILABLE_IN } from "~/shared/api";
import { AppShell, type Banner } from "~/widgets/app-shell";
import { sessionEnded } from "../../query/sessionEnded";
import { DemoNotice } from "../demo/DemoNotice";
import { usePageTitle } from "../focus/useDocumentTitle";
import { navGroups } from "../nav/navItems";

// Раздела справки пока нет, поэтому helpHref не передаём: пункт «Справка» не рисуется, а не ведёт на 404.
const DEFAULT_CRUMB = "CRANE";

/**
 * Шелл приложения. Навигация строится из `_links` ответа `GET /me` (решение D6) и показывает только пункты, для
 * которых в роутере есть маршрут. Плашка демо стоит над содержимым страницы, внутри `<main>`.
 */
export function ShellLayout() {
  const { t } = useLingui();
  const router = useRouter();
  const matchRoute = useMatchRoute();
  const ended = sessionEnded.use();
  const page = usePageTitle();
  // Данные положил в кэш beforeLoad шелла: отдельного запроса здесь нет.
  const { data } = useQuery(getGetMeQueryOptions());
  // Действия входа в баннере нет: входа до v0.4.0 нет (BFF), баннер называет версию.
  const version = SIGN_IN_AVAILABLE_IN.replace("v", "");
  const sessionEndedBanner: Banner = {
    id: "session-ended",
    tone: "warning",
    text: t`Сессия завершена. Вход в веб-консоль появится в CRANE ${version}.`,
  };
  const groups = data
    ? navGroups(data.data._links, {
        hasRoute: (to) => Object.hasOwn(router.routesByPath, to),
        isActive: (to) => matchRoute({ to, fuzzy: true }) !== false,
      })
    : [];
  return (
    <AppShell
      sidebar={{ groups, footerItems: [] }}
      topbar={{
        // Палитра подключится вместе с поиском: без обработчика кнопки поиска нет.
        breadcrumbs: { trail: [], current: page ?? DEFAULT_CRUMB },
      }}
      banners={ended ? [sessionEndedBanner] : []}
    >
      {DEMO ? <DemoNotice /> : null}
      <Outlet />
    </AppShell>
  );
}
