import { msg } from "@lingui/core/macro";
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
import { tr } from "./tr";

const UNREAD = 3;

/** Группы строятся снаружи из `_links` ответа `GET /me`, а не из названий ролей. */
const GROUPS = [
  {
    id: "catalog",
    label: msg`Каталог`,
    items: [
      { id: "search", label: msg`Поиск`, href: "/search", icon: SearchIcon },
      { id: "favorites", label: msg`Избранное`, href: "/favorites", icon: StarIcon },
    ],
  },
  {
    id: "mine",
    label: msg`Мои пакеты`,
    items: [{ id: "packages", label: msg`Мои пакеты`, href: "/packages", icon: PackageIcon }],
  },
  {
    id: "security",
    label: msg`Безопасность`,
    items: [
      { id: "findings", label: msg`Находки`, href: "/findings", icon: ShieldAlertIcon },
      { id: "exceptions", label: msg`Исключения`, href: "/exceptions", icon: ShieldOffIcon },
      { id: "vex", label: msg`VEX`, href: "/vex", icon: FileCheckIcon },
    ],
  },
  {
    id: "platform",
    label: msg`Платформа`,
    items: [
      { id: "overview", label: msg`Обзор`, href: "/platform", icon: LayoutDashboardIcon },
      { id: "storages", label: msg`Хранилища`, href: "/platform/storages", icon: DatabaseIcon },
      { id: "queues", label: msg`Очереди`, href: "/platform/queues", icon: ListChecksIcon },
    ],
  },
  {
    id: "journal",
    label: msg`Журнал`,
    items: [{ id: "audit", label: msg`Журнал`, href: "/audit", icon: ScrollTextIcon }],
  },
] as const;

const FOOTER = [
  { id: "tokens", label: msg`Мои токены`, href: "/me/tokens", icon: KeyRoundIcon },
  { id: "actions", label: msg`Мои действия`, href: "/me/actions", icon: HistoryIcon },
  { id: "settings", label: msg`Настройки`, href: "/settings", icon: SettingsIcon },
] as const;

type Props = {
  /** id активного пункта сайдбара (`search`, `packages`, `findings`, `overview`…). */
  active: string;
  breadcrumbs: TopbarProps["breadcrumbs"];
  banners?: readonly Banner[];
  children: ReactNode;
};

function sidebarFor(active: string): SidebarProps {
  return {
    groups: GROUPS.map((group) => ({
      id: group.id,
      label: tr(group.label),
      items: group.items.map((item) => ({
        ...item,
        label: tr(item.label),
        active: item.id === active,
      })),
    })),
    footerItems: FOOTER.map((item) => ({
      ...item,
      label: tr(item.label),
      active: item.id === active,
    })),
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
        {tr(msg`Workspace: ${workspace}`)}
      </Button>
    ),
    notifications: <NotificationsBell unread={UNREAD} />,
    account: (
      <Button variant="ghost" size="icon" aria-label={tr(msg`${initials}, аккаунт: ${name}`)}>
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
