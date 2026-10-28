import type { StubTable } from "./types";

// Загрузчики — инлайн-тернарники по константе сборки `__CRANE_UI_DEMO__` (не по импортированной `DEMO`: через границу
// модуля бандлер ветку не сворачивает). Отдельные константы нужны, чтобы запись таблицы осталась одной строкой, которую
// форматтер не разобьёт: линтер требует версию замены в строке сразу под пометкой.
const loadMe = __CRANE_UI_DEMO__ ? () => import("../data/me") : undefined;
const loadStatus = __CRANE_UI_DEMO__ ? () => import("../data/status") : undefined;

/**
 * Заглушки по operationId. Удалить запись = операция идёт в сеть, экраны не меняются. Над каждой записью стоит
 * пометка о замене с той же версией, что и `replacedIn`; версия не раньше `x-crane-since` операции.
 * Без демо загрузчики равны `undefined`, и `import()` вместе с модулем в dist не попадает.
 */
export const STUBS = {
  // FIXME(v0.4.0): getMe — профиль и навигация (_links) пока из заглушки, GET /me не подключён
  getMe: { replacedIn: "v0.4.0", load: loadMe },
  // FIXME(v0.4.0): getStatus — статус платформы пока из заглушки, GET /status не подключён
  getStatus: { replacedIn: "v0.4.0", load: loadStatus },
} satisfies StubTable;
