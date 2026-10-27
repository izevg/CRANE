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

type Crumb = { label: string; href: string };

export type TopbarProps = {
  /** Хлебные крошки живут только здесь: на странице второго ряда крошек нет. */
  breadcrumbs: {
    /** Путь до текущей страницы: каждая крошка — ссылка. */
    trail: readonly Crumb[];
    /** Текущая страница: ссылка у неё невыразима типом. */
    current: string;
  };
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

function Crumbs({ trail, current }: TopbarProps["breadcrumbs"]) {
  return (
    <Breadcrumb className="min-w-0">
      <BreadcrumbList className="flex-nowrap overflow-hidden">
        {trail.map((crumb) => (
          <Fragment key={`${crumb.label}:${crumb.href}`}>
            <BreadcrumbItem>
              <BreadcrumbLink href={crumb.href} className={CRUMB} title={crumb.label}>
                {crumb.label}
              </BreadcrumbLink>
            </BreadcrumbItem>
            <BreadcrumbSeparator />
          </Fragment>
        ))}
        <BreadcrumbItem>
          <BreadcrumbPage className={CRUMB} title={current}>
            {current}
          </BreadcrumbPage>
        </BreadcrumbItem>
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
      <Crumbs {...breadcrumbs} />
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
