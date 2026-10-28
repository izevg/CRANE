import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import {
  BuildingIcon,
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
import { i18n } from "~/shared/lib/i18n";
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
      label: i18n._(msg`Каталог`),
      items: [
        mark({ id: "search", label: i18n._(msg`Поиск`), href: "/search", icon: SearchIcon }),
        mark({
          id: "favorites",
          label: i18n._(msg`Избранное`),
          href: "/favorites",
          icon: StarIcon,
        }),
      ],
    },
    {
      id: "mine",
      label: i18n._(msg`Мои пакеты`),
      items: [
        mark({
          id: "packages",
          label: i18n._(msg`Мои пакеты`),
          href: "/packages",
          icon: PackageIcon,
        }),
      ],
    },
    {
      id: "security",
      label: i18n._(msg`Безопасность`),
      items: [
        mark({
          id: "findings",
          label: i18n._(msg`Находки`),
          href: "/findings",
          icon: ShieldAlertIcon,
        }),
        mark({
          id: "exceptions",
          label: i18n._(msg`Исключения`),
          href: "/exceptions",
          icon: ShieldOffIcon,
        }),
        mark({ id: "vex", label: i18n._(msg`VEX`), href: "/vex", icon: FileCheckIcon }),
      ],
    },
    {
      id: "platform",
      label: i18n._(msg`Платформа`),
      items: [
        mark({
          id: "overview",
          label: i18n._(msg`Обзор`),
          href: "/platform",
          icon: LayoutDashboardIcon,
        }),
        mark({
          id: "storages",
          label: i18n._(msg`Хранилища`),
          href: "/platform/storages",
          icon: DatabaseIcon,
        }),
        mark({
          id: "queues",
          label: i18n._(msg`Очереди`),
          href: "/platform/queues",
          icon: ListChecksIcon,
        }),
      ],
    },
    {
      id: "journal",
      label: i18n._(msg`Журнал`),
      items: [
        mark({ id: "audit", label: i18n._(msg`Журнал`), href: "/audit", icon: ScrollTextIcon }),
      ],
    },
  ];
}

function footerItems(active: string): SidebarProps["footerItems"] {
  const mark = <T extends { id: string }>(item: T) => ({ ...item, active: item.id === active });
  return [
    mark({ id: "tokens", label: i18n._(msg`Мои токены`), href: "/me/tokens", icon: KeyRoundIcon }),
    mark({
      id: "actions",
      label: i18n._(msg`Мои действия`),
      href: "/me/actions",
      icon: HistoryIcon,
    }),
    mark({ id: "settings", label: i18n._(msg`Настройки`), href: "/settings", icon: SettingsIcon }),
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
  const { t } = useLingui();
  const workspace = WORKSPACES[0];
  const initials = CURRENT_USER.initials;
  const name = CURRENT_USER.name;
  const topbar: TopbarProps = {
    breadcrumbs,
    onOpenPalette: () => undefined,
    workspace: (
      <Button variant="secondary" size="sm">
        <BuildingIcon aria-hidden="true" className="sm:hidden" />
        <span className="max-sm:sr-only">{t`Workspace: ${workspace}`}</span>
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
