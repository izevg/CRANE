import { polled } from "./polled";
import { type OperationMeta, stubbedFn } from "./stubbed";

type QueryLike = { queryFn?: unknown; refetchInterval?: unknown };

function isFunction(value: unknown): value is (arg: unknown) => unknown {
  return typeof value === "function";
}

/**
 * Мутатор `override.query.queryOptions` orval: единственная точка, где у операции известен operationId.
 * Подключает заглушку (`stubbed`) и каденс опроса (`polled`); экран не знает, настоящая операция или заглушка.
 */
export function withOperation<T extends QueryLike>(
  options: T,
  args: Readonly<Record<string, unknown>>,
  meta: OperationMeta,
): T {
  const { queryFn } = options;
  const stubbed = isFunction(queryFn)
    ? { ...options, queryFn: stubbedFn(queryFn, meta.operationId, () => args) }
    : options;
  return polled(stubbed, meta.operationId);
}
