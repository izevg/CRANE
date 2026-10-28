/**
 * Демо-режим — константа сборки (vite `define`), а не переключатель: true в `vite` dev, Storybook и тестах, а в
 * production только при `CRANE_UI_DEMO=1`. Для приложения; таблица заглушек читает глобальную константу напрямую.
 */
export const DEMO: boolean = __CRANE_UI_DEMO__;
