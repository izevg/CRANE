// biome-ignore-all lint/suspicious/noDeprecatedImports: помечена устаревшей только перегрузка t(i18n); форма ``t`…` `` из макроса Lingui 6 актуальна
import { t } from "@lingui/core/macro";
import {
  DatabaseIcon,
  FileCheckIcon,
  HistoryIcon,
  KeyRoundIcon,
  LayoutDashboardIcon,
  ListChecksIcon,
  PackageIcon,
  ScrollTextIcon,
  SearchIcon,
  SettingsIcon,
  ShieldAlertIcon,
  ShieldOffIcon,
  StarIcon,
} from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "~/shared/ui/primitives";
import { AppShell, type Banner, type SidebarProps, type TopbarProps } from "~/widgets/app-shell";
import { NotificationsBell } from "~/widgets/notifications-center";
import { CURRENT_USER, WORKSPACES } from "../data/common";

const UNREAD = 3;

/** Группы строятся снаружи из `_links` ответа `GET /me`, а не из названий ролей. Тексты берутся при рендере. */
function navGroups(active: string): SidebarProps["groups"] {
  const mark = <T extends { id: string }>(item: T) => ({ ...item, active: item.id === active });
  return [
    {
      id: "catalog",
      label: t`Каталог`,
      items: [
        mark({ id: "search", label: t`Поиск`, href: "/search", icon: SearchIcon }),
        mark({ id: "favorites", label: t`Избранное`, href: "/favorites", icon: StarIcon }),
      ],
    },
    {
      id: "mine",
      label: t`Мои пакеты`,
      items: [mark({ id: "packages", label: t`Мои пакеты`, href: "/packages", icon: PackageIcon })],
    },
    {
      id: "security",
      label: t`Безопасность`,
      items: [
        mark({ id: "findings", label: t`Находки`, href: "/findings", icon: ShieldAlertIcon }),
        mark({ id: "exceptions", label: t`Исключения`, href: "/exceptions", icon: ShieldOffIcon }),
        mark({ id: "vex", label: t`VEX`, href: "/vex", icon: FileCheckIcon }),
      ],
    },
    {
      id: "platform",
      label: t`Платформа`,
      items: [
        mark({ id: "overview", label: t`Обзор`, href: "/platform", icon: LayoutDashboardIcon }),
        mark({
          id: "storages",
          label: t`Хранилища`,
          href: "/platform/storages",
          icon: DatabaseIcon,
        }),
        mark({ id: "queues", label: t`Очереди`, href: "/platform/queues", icon: ListChecksIcon }),
      ],
    },
    {
      id: "journal",
      label: t`Журнал`,
      items: [mark({ id: "audit", label: t`Журнал`, href: "/audit", icon: ScrollTextIcon })],
    },
  ];
}

function footerItems(active: string): SidebarProps["footerItems"] {
  const mark = <T extends { id: string }>(item: T) => ({ ...item, active: item.id === active });
  return [
    mark({ id: "tokens", label: t`Мои токены`, href: "/me/tokens", icon: KeyRoundIcon }),
    mark({ id: "actions", label: t`Мои действия`, href: "/me/actions", icon: HistoryIcon }),
    mark({ id: "settings", label: t`Настройки`, href: "/settings", icon: SettingsIcon }),
  ];
}

type Props = {
  /** id активного пункта сайдбара (`search`, `packages`, `findings`, `overview`…). */
  active: string;
  breadcrumbs: TopbarProps["breadcrumbs"];
  banners?: readonly Banner[];
  children: ReactNode;
};

function sidebarFor(active: string): SidebarProps {
  return {
    groups: navGroups(active),
    footerItems: footerItems(active),
    helpHref: "/help",
    account: { name: CURRENT_USER.name, role: CURRENT_USER.role },
  };
}

/** Рамка экрана на реальном AppShell: сайдбар CRANE, topbar с палитрой, колокольчиком и аккаунтом. */
export function ScreenFrame({ active, breadcrumbs, banners, children }: Props) {
  const workspace = WORKSPACES[0];
  const initials = CURRENT_USER.initials;
  const name = CURRENT_USER.name;
  const topbar: TopbarProps = {
    breadcrumbs,
    onOpenPalette: () => undefined,
    workspace: (
      <Button variant="secondary" size="sm">
        {t`Workspace: ${workspace}`}
      </Button>
    ),
    notifications: <NotificationsBell unread={UNREAD} />,
    account: (
      <Button variant="ghost" size="icon" aria-label={t`${initials}, аккаунт: ${name}`}>
        {CURRENT_USER.initials}
      </Button>
    ),
  };
  return (
    <AppShell sidebar={sidebarFor(active)} topbar={topbar} {...(banners ? { banners } : {})}>
      {children}
    </AppShell>
  );
}
