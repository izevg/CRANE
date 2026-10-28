import { plannedProblem } from "../http/problem";
import { overrideFor } from "../stubs/override/override";
import { DEMO } from "../stubs/table/demo";
import { STUB_MODULES, STUBS } from "../stubs/table/table";
import type {
  OperationArgs,
  OperationId,
  StubHandler,
  StubModules,
  StubTable,
} from "../stubs/table/types";

/** Всё, от чего зависит решение «заглушка или сеть»: параметры, чтобы оба режима проверялись в одном прогоне. */
export type StubDeps = {
  demo: boolean;
  versions: StubTable;
  modules: StubModules;
  overrideFor: (operationId: string) => StubHandler | undefined;
};

const defaultStubDeps: StubDeps = {
  demo: DEMO,
  versions: STUBS,
  modules: STUB_MODULES,
  overrideFor,
};

export type OperationMeta = { url?: string; operationId: OperationId; operationName: string };

function isHandler(value: unknown): value is StubHandler {
  return typeof value === "function";
}

// Модули грузятся один раз на набор загрузчиков: в демо все данные приходят при первом заглушенном вызове.
const loaded = new WeakMap<StubModules, Promise<readonly Readonly<Record<string, unknown>>[]>>();

async function handlerOf(modules: StubModules, operationId: OperationId): Promise<StubHandler> {
  let all = loaded.get(modules);
  if (all === undefined) {
    all = Promise.all(Object.values(modules).map((load) => load()));
    loaded.set(modules, all);
  }
  for (const module of await all) {
    const handler = module[operationId];
    if (isHandler(handler)) {
      return handler;
    }
  }
  throw new Error(`Ни один модуль заглушек не экспортирует обработчик ${operationId}`);
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
  operationId: OperationId,
  inputOf: (arg: Arg) => OperationArgs,
  deps: StubDeps = defaultStubDeps,
): (arg: Arg) => Promise<unknown> {
  return async (arg) => {
    const input = inputOf(arg);
    const forced = deps.overrideFor(operationId);
    if (forced) {
      return respond(await forced(input));
    }
    const replacedIn = deps.versions[operationId];
    if (replacedIn === undefined) {
      return original(arg);
    }
    if (!deps.demo) {
      throw plannedProblem(replacedIn);
    }
    const handler = await handlerOf(deps.modules, operationId);
    return respond(await handler(input));
  };
}
