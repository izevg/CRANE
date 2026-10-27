import { Fragment, type ReactNode } from "react";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
} from "~/shared/ui/primitives";
import { SearchButton } from "./SearchButton";

const CRUMB = "block max-w-[24ch] truncate";

type Crumb = {
  label: string;
  /** У последней крошки (текущая страница) ссылки нет. */
  href?: string;
};

export type TopbarProps = {
  /** Хлебные крошки живут только здесь: на странице второго ряда крошек нет. */
  breadcrumbs: readonly Crumb[];
  onOpenPalette: () => void;
  workspace?: ReactNode;
  status?: ReactNode;
  notifications?: ReactNode;
  account?: ReactNode;
};

type Props = TopbarProps & {
  /** Кнопка сайдбара: состояние сайдбара принадлежит шеллу. */
  sidebarToggle: ReactNode;
};

function Crumbs({ crumbs }: { crumbs: readonly Crumb[] }) {
  return (
    <Breadcrumb className="min-w-0">
      <BreadcrumbList className="flex-nowrap overflow-hidden">
        {crumbs.map((crumb, index) => {
          const last = index === crumbs.length - 1;
          return (
            <Fragment key={`${crumb.label}:${crumb.href ?? ""}`}>
              <BreadcrumbItem>
                {last ? (
                  <BreadcrumbPage className={CRUMB} title={crumb.label}>
                    {crumb.label}
                  </BreadcrumbPage>
                ) : crumb.href !== undefined ? (
                  <BreadcrumbLink href={crumb.href} className={CRUMB} title={crumb.label}>
                    {crumb.label}
                  </BreadcrumbLink>
                ) : (
                  <span className={CRUMB} title={crumb.label}>
                    {crumb.label}
                  </span>
                )}
              </BreadcrumbItem>
              {last ? null : <BreadcrumbSeparator />}
            </Fragment>
          );
        })}
      </BreadcrumbList>
    </Breadcrumb>
  );
}

/** Верхняя панель 56 px. Порядок: сайдбар, крошки, поиск, workspace, статус, уведомления, аккаунт. */
export function Topbar({
  breadcrumbs,
  onOpenPalette,
  workspace,
  status,
  notifications,
  account,
  sidebarToggle,
}: Props) {
  return (
    <header className="sticky top-0 z-20 flex h-(--topbar-height) shrink-0 items-center gap-2 border-b border-border bg-background px-4 sm:gap-3 lg:px-6 2xl:px-8">
      {sidebarToggle}
      <Crumbs crumbs={breadcrumbs} />
      <div className="ml-auto flex shrink-0 items-center gap-2 sm:gap-3">
        <SearchButton onOpen={onOpenPalette} />
        {workspace}
        {status}
        {notifications}
        {account}
      </div>
    </header>
  );
}
