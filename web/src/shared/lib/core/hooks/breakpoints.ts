import { useMediaQuery } from "./useMediaQuery";

/**
 * Брейкпоинты Tailwind по умолчанию (px). Значения должны совпадать с `@theme` в app/styles/index.css:
 * в TS они нужны только там, где раскладка меняет структуру DOM (DetailPanel docked/Sheet, сайдбар/Sheet).
 * Остальная адаптивность делается CSS и container queries.
 */
export const BREAKPOINTS = { md: 768, lg: 1024, xl: 1280, "2xl": 1536 } as const;

export type Breakpoint = keyof typeof BREAKPOINTS;

/** `true`, когда ширина окна не меньше брейкпоинта (как префикс `md:` в Tailwind). */
export function useBreakpoint(breakpoint: Breakpoint): boolean {
  return useMediaQuery(`(min-width: ${BREAKPOINTS[breakpoint]}px)`);
}
