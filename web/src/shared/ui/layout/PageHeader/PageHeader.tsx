import { type ReactNode, type Ref, useId } from "react";
import { PageActionBar } from "./PageActionBar";
import type { PageActions } from "./pageActions";

type PageHeaderProps = PageActions & {
  /** Единственный h1 страницы: цель фокуса при смене маршрута (фокус переводит приложение, не кит). */
  title: string;
  /** Нужен, когда приложению известен id заранее (например, для `aria-labelledby`); иначе генерируется. */
  headingId?: string;
  headingRef?: Ref<HTMLHeadingElement>;
  /** StatusBadge. */
  status?: ReactNode;
  /** Чипы формата и репозитория. */
  chips?: ReactNode;
  caption?: string;
  /** Ряд StatChip. */
  stats?: ReactNode;
  /** Line tabs. */
  tabs?: ReactNode;
  /** Окно времени, AutoRefreshToggle: стоит рядом с действиями и не заменяет их. */
  aside?: ReactNode;
};

export function PageHeader({
  title,
  headingId,
  headingRef,
  status,
  chips,
  caption,
  stats,
  tabs,
  aside,
  ...actions
}: PageHeaderProps) {
  const generatedId = useId();
  return (
    <header data-slot="page-header" className="flex flex-col gap-3">
      <div className="flex flex-wrap items-start justify-between gap-x-4 gap-y-3">
        <div className="flex min-w-0 flex-col gap-1">
          <div className="flex flex-wrap items-center gap-x-3 gap-y-1">
            <h1
              ref={headingRef}
              id={headingId ?? generatedId}
              tabIndex={-1}
              className="text-balance wrap-anywhere text-h1 outline-none focus-visible:shadow-none"
            >
              {title}
            </h1>
            {status}
            {chips}
          </div>
          {caption ? (
            <p className="max-w-[72ch] wrap-anywhere text-body text-muted-foreground">{caption}</p>
          ) : null}
        </div>
        <div className="flex flex-wrap items-center gap-x-4 gap-y-2">
          {aside}
          <PageActionBar {...actions} />
        </div>
      </div>
      {stats ? <div className="flex flex-wrap gap-2">{stats}</div> : null}
      {tabs}
    </header>
  );
}
