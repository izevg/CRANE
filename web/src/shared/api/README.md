# shared/api: доступ к API и граница заглушек

Снаружи слоя виден только `index.ts` (правило dependency-cruiser `api-internals-are-private`). Всё остальное (`http`, `operation`, `stubs`, `generated`) — внутренности.

## Как экран получает данные

- Берите `get<Op>QueryOptions()` / `get<Op>MutationOptions()` из `~/shared/api` и передавайте один и тот же объект в loader маршрута (`ensureQueryData`) и в `useQuery`. Хуки `use<Op>` не используем; обёртка в entity нужна, только если она что-то добавляет (проекция, комбинация операций).
- Ответ имеет вид `{ data, status, headers }`, ошибка — `ApiProblem` (`kind`: `unauthenticated`, `step-up`, `csrf`, `problem`, `network`, `planned`). Разбор problem+json делает `http/problem.ts`, вид для `ProblemState` — `problemViewOf`.
- Транспорт (`http/fetcher.ts`): cookie той же страницы, GET `no-cache`, мутации `no-store` и `X-CSRF-Token`, один повтор после `csrf`. `Idempotency-Key` транспорт не ставит: ключ создаёт вызывающий и передаёт параметром операции.
- Каденс опроса (`operation/polled.ts`) — свойство операции, не вызова; пауза — один флаг в `shared/lib/state`.

## Заглушки

- Таблица `stubs/table/table.ts`: ключ — operationId (тип выводится из спеки), значение `{ replacedIn, load }`. Над записью `// FIXME(vX.Y.Z): <operationId> <что заглушено>`, версия в комментарии равна `replacedIn` и не раньше `x-crane-since`. Удалить запись — операция идёт в сеть, экраны не меняются.
- Модуль данных в `stubs/data/` экспортирует `STUB_MARKER` (`crane-stub:<имя>`) и обработчики по operationId. Шаг `tools/check-bundle.ts` ищет маркер в `dist` и валит сборку без демо-флага.
- Демо — константа сборки `__CRANE_UI_DEMO__` (vite `define`): true в dev, Storybook и тестах, в production только при `CRANE_UI_DEMO=1`. Загрузчик пишется инлайн-тернарником по этой константе, иначе бандлер не выбросит ветку.
- Без демо заглушенная операция в сеть не идёт: `queryFn` бросает `ApiProblem` `planned` с версией замены, страница показывает «Раздел появится в vX.Y.Z».
- Storybook: `parameters.api = { <operationId>: () => тело | бросок | вечный промис } satisfies ApiOverrides`; декоратор в `.storybook/preview.tsx` сбрасывает подмены перед каждой историей.
