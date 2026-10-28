import { type OperationMeta, stubbedFn } from "./stubbed";

function isFunction(value: unknown): value is (arg: unknown) => unknown {
  return typeof value === "function";
}

function isArgs(value: unknown): value is Readonly<Record<string, unknown>> {
  return typeof value === "object" && value !== null;
}

/**
 * Мутатор `override.query.mutationOptions` orval. Входы заглушки мутации — переменные вызова, а не аргументы
 * фабрики опций, поэтому берутся в момент выполнения.
 */
export function withMutation<T extends { mutationFn?: unknown }>(
  options: T,
  _route: { url: string },
  meta: OperationMeta,
): T {
  const { mutationFn } = options;
  if (!isFunction(mutationFn)) {
    return options;
  }
  return {
    ...options,
    mutationFn: stubbedFn(mutationFn, meta.operationId, (variables: unknown) =>
      isArgs(variables) ? variables : {},
    ),
  };
}
