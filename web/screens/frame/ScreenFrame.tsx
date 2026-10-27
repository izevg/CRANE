// i18n: не мигрирован
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
const WORKSPACE = `Workspace: ${WORKSPACES[0]}`;
const ACCOUNT_LABEL = `${CURRENT_USER.initials}, аккаунт: ${CURRENT_USER.name}`;
const ACCOUNT_BLOCK = `${CURRENT_USER.name}, ${CURRENT_USER.role}`;

/** Группы строятся снаружи из `_links` ответа `GET /me`, а не из названий ролей. */
const GROUPS = [
  {
    id: "catalog",
    label: "Каталог",
    items: [
      { id: "search", label: "Поиск", href: "/search", icon: SearchIcon },
      { id: "favorites", label: "Избранное", href: "/favorites", icon: StarIcon },
    ],
  },
  {
    id: "mine",
    label: "Мои пакеты",
    items: [{ id: "packages", label: "Мои пакеты", href: "/packages", icon: PackageIcon }],
  },
  {
    id: "security",
    label: "Безопасность",
    items: [
      { id: "findings", label: "Находки", href: "/findings", icon: ShieldAlertIcon },
      { id: "exceptions", label: "Исключения", href: "/exceptions", icon: ShieldOffIcon },
      { id: "vex", label: "VEX", href: "/vex", icon: FileCheckIcon },
    ],
  },
  {
    id: "platform",
    label: "Платформа",
    items: [
      { id: "overview", label: "Обзор", href: "/platform", icon: LayoutDashboardIcon },
      { id: "storages", label: "Хранилища", href: "/platform/storages", icon: DatabaseIcon },
      { id: "queues", label: "Очереди", href: "/platform/queues", icon: ListChecksIcon },
    ],
  },
  {
    id: "journal",
    label: "Журнал",
    items: [{ id: "audit", label: "Журнал", href: "/audit", icon: ScrollTextIcon }],
  },
] as const;

const FOOTER = [
  { id: "tokens", label: "Мои токены", href: "/me/tokens", icon: KeyRoundIcon },
  { id: "actions", label: "Мои действия", href: "/me/actions", icon: HistoryIcon },
  { id: "settings", label: "Настройки", href: "/settings", icon: SettingsIcon },
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
      ...group,
      items: group.items.map((item) => ({ ...item, active: item.id === active })),
    })),
    footerItems: FOOTER.map((item) => ({ ...item, active: item.id === active })),
    helpHref: "/help",
    account: (
      <Button variant="ghost" className="w-full justify-start">
        {ACCOUNT_BLOCK}
      </Button>
    ),
  };
}

/** Рамка экрана на реальном AppShell: сайдбар CRANE, topbar с палитрой, колокольчиком и аккаунтом. */
export function ScreenFrame({ active, breadcrumbs, banners, children }: Props) {
  const topbar: TopbarProps = {
    breadcrumbs,
    onOpenPalette: () => undefined,
    workspace: (
      <Button variant="secondary" size="sm">
        {WORKSPACE}
      </Button>
    ),
    notifications: <NotificationsBell unread={UNREAD} />,
    account: (
      <Button variant="ghost" size="icon" aria-label={ACCOUNT_LABEL}>
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
