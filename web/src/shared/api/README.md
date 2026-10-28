# shared/api: доступ к API и граница заглушек

Снаружи слоя виден только `index.ts` (правило dependency-cruiser `api-internals-are-private`). Всё остальное (`http`, `operation`, `stubs`, `generated`) — внутренности.

## Как экран получает данные

- Берите `get<Op>QueryOptions()` / `get<Op>MutationOptions()` из `~/shared/api` и передавайте один и тот же объект в loader маршрута (`ensureQueryData`) и в `useQuery`. Хуки `use<Op>` не используем; обёртка в entity нужна, только если она что-то добавляет (проекция, комбинация операций).
- Данные страницы читает `useSuspenseQuery(get<Op>SuspenseQueryOptions())` (опции без `skipToken`), loader — `ensureQueryData(get<Op>QueryOptions())` с тем же ключом.
- Ответ имеет вид `{ data, status, headers }`, ошибка — `ApiProblem` (`kind`: `unauthenticated`, `step-up`, `csrf`, `problem`, `network`, `planned`). Разбор problem+json делает `http/problem.ts`, вид для `ProblemState` — `problemViewOf`. Тип ошибки в сгенерированных опциях — `ApiProblem` (`ErrorType` в `http/fetcher.ts`), сужать его через `instanceof` не нужно.
- Транспорт (`http/fetcher.ts`): cookie той же страницы, GET `no-cache`, мутации `no-store` и `X-CSRF-Token`, один повтор после `csrf`. `Idempotency-Key` транспорт не ставит: ключ создаёт вызывающий и передаёт параметром операции.
- Каденс опроса (`operation/polled.ts`) — свойство операции, не вызова; пауза — один флаг в `shared/lib/state`.

## Заглушки

- Таблица `stubs/table/table.ts`: ключ — operationId (тип выводится из спеки), значение — версия, в которой операция становится настоящей. Над записью `// FIXME(vX.Y.Z): <operationId> <что заглушено>`, версия в комментарии равна литералу в строке записи и не раньше `x-crane-since`. Удалить запись — операция идёт в сеть, экраны не меняются.
- Модуль данных в `stubs/data/` экспортирует обработчики по operationId (`export function getStatus(): Status`). Подключать его никуда не нужно: `import.meta.glob` в `table.ts` находит все модули. Шаг `tools/check-bundle.ts` смотрит в `manifest.json`: без демо-флага ключей `src/shared/api/stubs/data/` быть не должно.
- Демо — константа сборки `__CRANE_UI_DEMO__` (vite `define`): true в dev, Storybook и тестах, в production только при `CRANE_UI_DEMO=1`. Условие читает именно её, иначе бандлер не выбросит ветку с модулями.
- Без демо заглушенная операция в сеть не идёт: `queryFn` бросает `ApiProblem` `planned` с версией замены, страница показывает «Раздел пока недоступен».
- Storybook: `parameters.api = { <operationId>: () => тело | бросок | вечный промис } satisfies ApiOverrides`; загрузчик в `.storybook/preview.tsx` сбрасывает подмены перед каждой историей.
