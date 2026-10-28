import type { StubModules, StubTable } from "./types";

/**
 * Заглушки по operationId: версия, в которой операция становится настоящей. Удалить запись = операция идёт в сеть,
 * экраны не меняются. Над каждой записью стоит пометка о замене с той же версией (она не раньше `x-crane-since`).
 */
export const STUBS = {
  // FIXME(v0.4.0): getMe — профиль и навигация (_links) пока из заглушки, GET /me не подключён
  getMe: "v0.4.0",
  // FIXME(v0.4.0): getStatus — статус платформы пока из заглушки, GET /status не подключён
  getStatus: "v0.4.0",
} satisfies StubTable;

/**
 * Модули данных заглушек. Условие читает глобальную константу сборки `__CRANE_UI_DEMO__` прямо здесь (не через
 * импортированную `DEMO`: границу модуля бандлер не сворачивает): без демо ветка с `import()` выбрасывается, и
 * модули данных в dist не попадают. Обработчик ищется по operationId среди экспортов модулей.
 */
// Stryker disable all: Vite раскрывает import.meta.glob по литералу до выполнения, подставленная мутантом строка ломает разбор
export const STUB_MODULES: StubModules = __CRANE_UI_DEMO__
  ? import.meta.glob<Record<string, unknown>>("../data/*.ts")
  : {};
// Stryker restore all
