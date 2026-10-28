import { useLingui } from "@lingui/react/macro";
import { CircleHelpIcon, DotIcon, type LucideIcon } from "lucide-react";
import { useId } from "react";
import { cx } from "~/shared/lib/core";
import { Link, Tooltip, TooltipContent, TooltipTrigger } from "~/shared/ui/primitives";
import { initialsOf } from "../../lib/sidebar";
import { LocaleMenu } from "../locale/LocaleMenu";
import { Logo } from "./Logo";
import { ThemeMenu } from "./ThemeMenu";

const DEFAULT_HOME = "/";

type NavItem = {
  id: string;
  label: string;
  href: string;
  icon?: LucideIcon;
  active?: boolean;
};

type NavGroup = {
  id: string;
  label?: string;
  items: readonly NavItem[];
};

export type SidebarProps = {
  /** Группы строятся снаружи из `_links` ответа `GET /me`, а не из названий ролей. */
  groups: readonly NavGroup[];
  /** «Мои токены», «Мои действия», «Настройки». «Справка» добавляется сайдбаром сама. */
  footerItems: readonly NavItem[];
  /**
   * Справка стоит в одном месте нижнего блока (WCAG 3.2.6). Пока раздела справки нет, ссылку не передают:
   * пункта тогда нет вовсе, а не ссылка на 404.
   */
  helpHref?: string;
  homeHref?: string;
  /** Текущий пользователь: две строки в развёрнутом сайдбаре, только аватар в рельсе. */
  account?: { name: string; role: string };
};

type ContentProps = SidebarProps & {
  collapsed: boolean;
  /** Закрывает Sheet после перехода по ссылке. */
  onNavigate?: () => void;
};

const ITEM_BASE =
  "flex h-10 items-center gap-3 rounded-md px-2.5 text-body transition-colors duration-(--dur-fast) hover:bg-sidebar-accent";
const ITEM_ACTIVE = "bg-sidebar-accent font-semibold text-sidebar-accent-foreground";

type ItemProps = { item: NavItem; collapsed: boolean; onNavigate: (() => void) | undefined };

/** В рельсе подпись остаётся в DOM (sr-only) и дублируется подсказкой справа. */
function NavLinkItem({ item, collapsed, onNavigate }: ItemProps) {
  const Icon = item.icon ?? DotIcon;
  const link = (
    <Link
      href={item.href}
      aria-current={item.active ? "page" : undefined}
      onClick={onNavigate}
      className={cx(ITEM_BASE, item.active && ITEM_ACTIVE, collapsed && "justify-center")}
    >
      <Icon aria-hidden="true" className="size-4 shrink-0" />
      <span className={collapsed ? "sr-only" : "truncate"}>{item.label}</span>
    </Link>
  );
  if (!collapsed) {
    return <li>{link}</li>;
  }
  return (
    <li>
      <Tooltip>
        <TooltipTrigger render={link} />
        <TooltipContent side="right">{item.label}</TooltipContent>
      </Tooltip>
    </li>
  );
}

function NavGroupList({
  group,
  collapsed,
  onNavigate,
}: Omit<ItemProps, "item"> & { group: NavGroup }) {
  const labelId = useId();
  const labelled = group.label !== undefined;
  return (
    <div
      className={
        collapsed
          ? "border-t border-sidebar-border pt-2 first:border-t-0 first:pt-0"
          : "pt-4 first:pt-0"
      }
    >
      {labelled ? (
        <p
          id={labelId}
          className={
            collapsed ? "sr-only" : "px-2.5 pb-1 text-caption font-semibold text-muted-foreground"
          }
        >
          {group.label}
        </p>
      ) : null}
      <ul aria-labelledby={labelled ? labelId : undefined} className="flex flex-col gap-0.5">
        {group.items.map((item) => (
          <NavLinkItem key={item.id} item={item} collapsed={collapsed} onNavigate={onNavigate} />
        ))}
      </ul>
    </div>
  );
}

/**
 * Блок пользователя. Длинные имя и роль обрезаются многоточием (полный текст в `title`), блок не шире сайдбара.
 * В рельсе остаётся только аватар: его доступное имя содержит и имя, и роль.
 */
function SidebarUser({
  name,
  role,
  collapsed,
}: {
  name: string;
  role: string;
  collapsed: boolean;
}) {
  const avatar = (
    <span
      className="flex size-8 shrink-0 items-center justify-center rounded-full bg-brand-container text-caption font-semibold text-brand"
      {...(collapsed
        ? { role: "img", "aria-label": `${name}, ${role}`, title: `${name}, ${role}` }
        : { "aria-hidden": true })}
    >
      {initialsOf(name)}
    </span>
  );
  return (
    <div
      data-slot="sidebar-user"
      className={cx(
        "flex min-w-0 items-center gap-3 px-2.5 py-2",
        collapsed && "justify-center px-0",
      )}
    >
      {avatar}
      {collapsed ? null : (
        <div className="flex min-w-0 flex-1 flex-col">
          <span title={name} className="truncate text-body font-semibold">
            {name}
          </span>
          <span title={role} className="truncate text-caption text-muted-foreground">
            {role}
          </span>
        </div>
      )}
    </div>
  );
}

/** Содержимое сайдбара: общее для боковой колонки (248/64 px) и для Sheet на узком экране. */
export function Sidebar({
  groups,
  footerItems,
  helpHref,
  homeHref = DEFAULT_HOME,
  account,
  collapsed,
  onNavigate,
}: ContentProps) {
  const { t } = useLingui();
  const help: readonly NavItem[] =
    helpHref === undefined
      ? []
      : [{ id: "help", label: t`Справка`, href: helpHref, icon: CircleHelpIcon }];
  return (
    <div
      data-slot="sidebar"
      className="flex h-full flex-col overflow-x-hidden bg-sidebar text-sidebar-foreground"
    >
      <div
        className={cx(
          "flex h-(--topbar-height) shrink-0 items-center border-b border-sidebar-border",
          collapsed ? "justify-center" : "px-4",
        )}
      >
        <Logo href={homeHref} collapsed={collapsed} />
      </div>
      <nav aria-label={t`Основное меню`} className="min-h-0 flex-1 overflow-y-auto p-3">
        {groups.map((group) => (
          <NavGroupList
            key={group.id}
            group={group}
            collapsed={collapsed}
            onNavigate={onNavigate}
          />
        ))}
      </nav>
      <div className="flex shrink-0 flex-col gap-1 border-t border-sidebar-border p-3">
        <nav aria-label={t`Личное и справка`}>
          <ul className="flex flex-col gap-0.5">
            {[...footerItems, ...help].map((item) => (
              <NavLinkItem
                key={item.id}
                item={item}
                collapsed={collapsed}
                onNavigate={onNavigate}
              />
            ))}
          </ul>
        </nav>
        <LocaleMenu collapsed={collapsed} />
        <ThemeMenu collapsed={collapsed} />
        {account ? <SidebarUser {...account} collapsed={collapsed} /> : null}
      </div>
    </div>
  );
}
