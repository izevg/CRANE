import { plannedProblem } from "../http/problem";
import { overrideFor } from "../stubs/override/override";
import { DEMO } from "../stubs/table/demo";
import { STUBS } from "../stubs/table/table";
import type { OperationArgs, StubEntries } from "../stubs/table/types";

type Handler = (args: OperationArgs) => unknown;

/** Всё, от чего зависит решение «заглушка или сеть»: параметры, чтобы оба режима проверялись в одном прогоне. */
export type StubDeps = {
  demo: boolean;
  entries: StubEntries;
  overrideFor: (operationId: string) => Handler | undefined;
};

const defaultStubDeps: StubDeps = { demo: DEMO, entries: STUBS, overrideFor };

export type OperationMeta = { url?: string; operationId: string; operationName: string };

function isHandler(value: unknown): value is Handler {
  return typeof value === "function";
}

/** Ответ заглушки в той же форме, что у транспорта: `{ data, status, headers }`. */
function respond(data: unknown): { data: unknown; status: number; headers: Headers } {
  return { data, status: 200, headers: new Headers() };
}

/**
 * Подменяет выполнение операции. Порядок: подмена Storybook, затем таблица заглушек. Операции вне таблицы идут в
 * сеть. Операция из таблицы без демо в сеть НЕ идёт и данных не отдаёт: бросает `planned` с версией замены.
 * Решение принимается при вызове, а не при создании опций: подмены между историями Storybook сбрасываются.
 */
export function stubbedFn<Arg>(
  original: (arg: Arg) => unknown,
  operationId: string,
  inputOf: (arg: Arg) => OperationArgs,
  deps: StubDeps = defaultStubDeps,
): (arg: Arg) => Promise<unknown> {
  return async (arg) => {
    const input = inputOf(arg);
    const forced = deps.overrideFor(operationId);
    if (forced) {
      return respond(await forced(input));
    }
    const entry = deps.entries[operationId];
    if (entry === undefined) {
      return original(arg);
    }
    if (!deps.demo || entry.load === undefined) {
      throw plannedProblem(entry.replacedIn);
    }
    const handler = (await entry.load())[operationId];
    if (!isHandler(handler)) {
      throw new Error(`Модуль заглушки не экспортирует обработчик ${operationId}`);
    }
    return respond(await handler(input));
  };
}
