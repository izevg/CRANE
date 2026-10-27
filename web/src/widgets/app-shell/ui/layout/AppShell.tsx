// i18n: не мигрирован
import { type MouseEvent, type ReactNode, useCallback, useRef, useState } from "react";
import { cx, useBreakpoint } from "~/shared/lib/core";
import { Sheet, SheetContent, SheetDescription, SheetTitle } from "~/shared/ui/primitives";
import { type Banner, BannerStack } from "../banners/BannerStack";
import { Sidebar, type SidebarProps } from "../sidebar/Sidebar";
import { type SidebarMode, SidebarToggle } from "../sidebar/SidebarToggle";
import { Topbar, type TopbarProps } from "../topbar/Topbar";

const SKIP_LABEL = "К содержимому";
const MAIN_ID = "main-content";
const MENU_TITLE = "Главное меню";
const MENU_DESCRIPTION = "Навигация по разделам реестра";

type AppShellProps = {
  sidebar: SidebarProps;
  topbar: TopbarProps;
  /** Уже отсортирован по важности. */
  banners?: readonly Banner[];
  /** Свернуть info-баннер; без обработчика кнопки «Свернуть» нет. */
  onCollapseBanner?: (id: string) => void;
  children: ReactNode;
};

/**
 * ≥ lg — колонка 248 px (или рельса 64 px по выбору пользователя), md–lg — рельса принудительно,
 * ниже md — Sheet. Режим зависит от ширины, поэтому он меняет структуру DOM, а не только CSS.
 */
function useSidebarMode(collapsed: boolean): { mode: SidebarMode; canToggle: boolean } {
  const wide = useBreakpoint("lg");
  const medium = useBreakpoint("md");
  if (!medium) {
    return { mode: "sheet", canToggle: true };
  }
  // На md–lg рельса принудительная: кнопки свёртки нет, развернуть некуда.
  return { mode: wide && !collapsed ? "full" : "rail", canToggle: wide };
}

/** Корень всех страниц: skip-link, сайдбар, topbar, зона баннеров и контент с `@container/page`. */
export function AppShell({
  sidebar,
  topbar,
  banners = [],
  onCollapseBanner,
  children,
}: AppShellProps) {
  const [collapsed, setCollapsed] = useState(false);
  const [sheetOpen, setSheetOpen] = useState(false);
  const { mode, canToggle } = useSidebarMode(collapsed);
  const mainRef = useRef<HTMLElement>(null);

  const skipToMain = useCallback((event: MouseEvent<HTMLAnchorElement>) => {
    // Без перехода по якорю: hash-роутер принял бы его за смену маршрута.
    event.preventDefault();
    mainRef.current?.focus();
  }, []);
  const toggle = useCallback(() => {
    if (mode === "sheet") {
      setSheetOpen((value) => !value);
    } else {
      setCollapsed((value) => !value);
    }
  }, [mode]);
  const closeSheet = useCallback(() => setSheetOpen(false), []);

  return (
    <div className="flex min-h-svh bg-background text-foreground">
      <a
        href={`#${MAIN_ID}`}
        onClick={skipToMain}
        className="sr-only focus:not-sr-only focus:fixed focus:top-2 focus:left-2 focus:z-50 focus:rounded-md focus:bg-card focus:px-4 focus:py-2 focus:text-body focus:text-foreground focus:shadow-card"
      >
        {SKIP_LABEL}
      </a>
      {mode === "sheet" ? (
        <Sheet open={sheetOpen} onOpenChange={setSheetOpen}>
          <SheetContent side="left" className="gap-0 p-0">
            <SheetTitle className="sr-only">{MENU_TITLE}</SheetTitle>
            <SheetDescription className="sr-only">{MENU_DESCRIPTION}</SheetDescription>
            <Sidebar {...sidebar} collapsed={false} onNavigate={closeSheet} />
          </SheetContent>
        </Sheet>
      ) : (
        <div
          className={cx(
            "sticky top-0 h-svh shrink-0 border-r border-sidebar-border",
            mode === "full" ? "w-62" : "w-16",
          )}
        >
          <Sidebar {...sidebar} collapsed={mode === "rail"} />
        </div>
      )}
      <div className="flex min-w-0 flex-1 flex-col">
        <Topbar
          {...topbar}
          sidebarToggle={
            canToggle ? <SidebarToggle mode={mode} sheetOpen={sheetOpen} onToggle={toggle} /> : null
          }
        />
        <BannerStack banners={banners} onCollapse={onCollapseBanner} />
        <main
          ref={mainRef}
          id={MAIN_ID}
          tabIndex={-1}
          className="@container/page mx-auto w-full max-w-[1600px] flex-1 px-4 py-6 outline-none lg:px-6 2xl:px-8"
        >
          {children}
        </main>
      </div>
    </div>
  );
}
