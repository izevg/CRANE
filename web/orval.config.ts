import { defineConfig } from "orval";

/**
 * Клиент: react-query поверх fetch. Страницы берут `get<Op>QueryOptions` напрямую (один объект в loader маршрута и
 * в useQuery), хуки `use<Op>` не используются, а сырые request-функции наружу не экспортируются: заглушки
 * (`shared/api/stubs`) подключаются в мутаторах опций, и обойти их вызовом в обход Query было бы нечем.
 * Сгенерированный файл не коммитится; в сборку попадает только то, что импортировано (tree-shaking).
 */
export default defineConfig({
  admin: {
    input: {
      target: "../docs/api/admin.openapi.yaml",
      parserOptions: { externalRefs: { allow: ["./common.yaml"] } },
    },
    output: {
      target: "./src/shared/api/generated/admin.gen.ts",
      mode: "single",
      schemas: "./src/shared/api/generated/schemas",
      operationManifest: { path: "./src/shared/api/generated/operations.gen.ts", types: false },
      client: "react-query",
      httpClient: "fetch",
      clean: true,
      // Idempotency-Key и подобные заголовки операции попадают в сигнатуру: ключ создаёт вызывающий, а не транспорт.
      headers: true,
      override: {
        // Файл не коммитится, пометка «не править» не нужна; union вместо const-объектов: словарям нужен только тип ключа.
        header: false,
        enumGenerationType: "union",
        mutator: { path: "./src/shared/api/http/fetcher.ts", name: "fetcher" },
        // Без этого не-2xx приходит значением, а не бросается, и isError у Query не срабатывает.
        fetch: { forceSuccessResponse: true },
        query: {
          // Страницы читают данные через useSuspenseQuery: ему нужны опции без skipToken и enabled.
          useSuspenseQuery: true,
          queryOptions: {
            path: "./src/shared/api/operation/queryOptions.ts",
            name: "withOperation",
            // Без хуков имя функции get<Op>QueryOptions, а не use<Op>QueryOptions: это не хук, его зовут и из loader.
            useHooks: false,
          },
          mutationOptions: {
            path: "./src/shared/api/operation/mutationOptions.ts",
            name: "withMutation",
            useHooks: false,
          },
          shouldExportHttpClient: false,
          // Отмена запроса при смене фильтров доходит до fetch.
          signal: true,
          // Ключ — сегменты пути: префиксная инвалидация ресурса вместе с подресурсами.
          shouldSplitQueryKey: true,
        },
      },
    },
  },
});
