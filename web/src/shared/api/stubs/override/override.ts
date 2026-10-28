import type { OperationId, StubHandler } from "../table/types";

/** Подмены ответов по operationId: в параметрах истории Storybook (`parameters.api`). */
export type ApiOverrides = { [K in OperationId]?: StubHandler };

const handlers = new Map<string, StubHandler>();

/**
 * Подмена ответа операции в Storybook: сильнее таблицы заглушек и работает при любом демо-флаге.
 * Хэндлер может бросить `ApiProblem` (состояние ошибки) или вернуть промис, который не завершается (загрузка).
 */
export function override(operationId: OperationId, handler: StubHandler): void {
  handlers.set(operationId, handler);
}

/** Декоратор Storybook вызывает перед каждой историей: подмены одной истории не переходят в другую. */
export function resetOverrides(): void {
  handlers.clear();
}

function isHandler(value: unknown): value is StubHandler {
  return typeof value === "function";
}

function isOverrides(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/**
 * Регистрирует подмены из параметров истории после сброса предыдущих. Параметры приходят из Storybook без типов,
 * поэтому значения проверяются: не функция — ошибка истории, а не молчаливая потеря подмены.
 */
export function applyOverrides(parameters: unknown): void {
  resetOverrides();
  if (!isOverrides(parameters)) {
    return;
  }
  for (const [operationId, handler] of Object.entries(parameters)) {
    if (!isHandler(handler)) {
      throw new Error(`Подмена ${operationId} в parameters.api должна быть функцией`);
    }
    handlers.set(operationId, handler);
  }
}

export function overrideFor(operationId: string): StubHandler | undefined {
  return handlers.get(operationId);
}
