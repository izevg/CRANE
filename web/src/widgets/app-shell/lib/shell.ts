import { useBreakpoint } from "~/shared/lib/core";

/** `sheet` — меню в Sheet, `full` — развёрнутая колонка 248 px, `rail` — рельса 64 px. */
export type SidebarMode = "full" | "rail" | "sheet";

/**
 * ≥ lg — колонка 248 px (или рельса 64 px по выбору пользователя), md–lg — рельса принудительно,
 * ниже md — Sheet. Режим зависит от ширины, поэтому он меняет структуру DOM, а не только CSS.
 */
export function useSidebarMode(collapsed: boolean): { mode: SidebarMode; canToggle: boolean } {
  const wide = useBreakpoint("lg");
  const medium = useBreakpoint("md");
  if (!medium) {
    return { mode: "sheet", canToggle: true };
  }
  // На md–lg рельса принудительная: кнопки свёртки нет, развернуть некуда.
  return { mode: wide && !collapsed ? "full" : "rail", canToggle: wide };
}
