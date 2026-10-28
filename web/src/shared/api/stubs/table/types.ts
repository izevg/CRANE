import type { operations } from "../../generated/operations.gen";

/**
 * Все operationId спеки: ключи манифеста операций, который генерирует orval. Манифест не импортирует ни мутаторы,
 * ни клиент, поэтому тип не замыкает цикл «клиент → мутатор → таблица заглушек → клиент». Операция, удалённая
 * из спеки, ломает сборку записи заглушки, опечатка в ключе не компилируется.
 */
export type OperationId = keyof typeof operations;

/** Входы операции: второй аргумент мутатора orval (`{ headers }`, `{ params }`, `{ data }`). */
export type OperationArgs = Readonly<Record<string, unknown>>;

/**
 * Обработчик заглушки возвращает тело успешного ответа (без обёртки `{ data, status, headers }`). Его форму
 * держит аннотация возвращаемого типа в модуле данных и тест таблицы против сгенерированных типов.
 */
export type StubHandler = (args: OperationArgs) => unknown;

/** Модуль данных заглушки: именованные обработчики по operationId. */
type StubModule<K extends OperationId> = { readonly [P in K]: StubHandler };

type Version = `v${number}.${number}.${number}`;

type StubEntry<K extends OperationId> = {
  /** Версия, в которой операция становится настоящей (не раньше `x-crane-since`). */
  replacedIn: Version;
  /** Загрузчик данных; `undefined` — демо выключено, данных нет вовсе. */
  load: (() => Promise<StubModule<K>>) | undefined;
};

export type StubTable = { [K in OperationId]?: StubEntry<K> };

/** Вид таблицы для выполнения: ключи уже проверены типом `StubTable`, обработчик ищется по имени. */
export type StubEntries = Readonly<
  Partial<
    Record<
      string,
      { replacedIn: string; load: (() => Promise<Readonly<Record<string, unknown>>>) | undefined }
    >
  >
>;
