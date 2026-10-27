import { CircleHelpIcon, DotIcon, type LucideIcon } from "lucide-react";
import { type ReactNode, useId } from "react";
import { cx } from "~/shared/lib/core";
import { Link, Tooltip, TooltipContent, TooltipTrigger } from "~/shared/ui/primitives";
import { Logo } from "./Logo";
import { ThemeMenu } from "./ThemeMenu";

const MAIN_NAV = "Основное меню";
const SERVICE_NAV = "Личное и справка";
const HELP = "Справка";
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
  /** Справка всегда стоит в одном месте нижнего блока (WCAG 3.2.6), поэтому ссылка обязательна. */
  helpHref: string;
  homeHref?: string;
  account?: ReactNode;
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
  const help: NavItem = { id: "help", label: HELP, href: helpHref, icon: CircleHelpIcon };
  return (
    <div className="flex h-full flex-col bg-sidebar text-sidebar-foreground">
      <div
        className={cx(
          "flex h-(--topbar-height) shrink-0 items-center border-b border-sidebar-border",
          collapsed ? "justify-center" : "px-4",
        )}
      >
        <Logo href={homeHref} collapsed={collapsed} />
      </div>
      <nav aria-label={MAIN_NAV} className="min-h-0 flex-1 overflow-y-auto p-3">
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
        <nav aria-label={SERVICE_NAV}>
          <ul className="flex flex-col gap-0.5">
            {[...footerItems, help].map((item) => (
              <NavLinkItem
                key={item.id}
                item={item}
                collapsed={collapsed}
                onNavigate={onNavigate}
              />
            ))}
          </ul>
        </nav>
        <ThemeMenu collapsed={collapsed} />
        {account}
      </div>
    </div>
  );
}
