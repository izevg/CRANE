import { defineConfig } from "orval";

/**
 * Пока генерируются только типы схем. Клиент и data fetching придут с TanStack Query (раздел 15, v0.4.0).
 * Режима «только типы» у orval нет, поэтому операции отсекает фильтр по тегу, которого в спеке нет.
 * Включить клиент = убрать `filters`. Валидация спеки работает и с фильтром.
 */
export default defineConfig({
  admin: {
    input: {
      target: "../docs/api/admin.openapi.yaml",
      parserOptions: { externalRefs: { allow: ["./common.yaml"] } },
      filters: { mode: "include", tags: ["types-only"], includeUnreferencedSchemas: true },
    },
    output: {
      target: "./src/shared/api/generated/admin.gen.ts",
      mode: "single",
      // Без операций клиент не даёт ни одного import; axios по умолчанию потянул бы зависимость.
      client: "fetch",
      clean: true,
      // Файл не коммитится, пометка «не править» не нужна; union вместо const-объектов: словарям нужен только тип ключа.
      override: { header: false, enumGenerationType: "union" },
    },
  },
});
