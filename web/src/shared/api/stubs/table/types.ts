import type { operations } from "../../generated/operations.gen";

/**
 * Все operationId спеки: ключи манифеста операций, который генерирует orval. Манифест не импортирует ни мутаторы,
 * ни клиент, поэтому тип не замыкает цикл «клиент → мутатор → таблица заглушек → клиент». Операция, удалённая
 * из спеки, ломает сборку записи заглушки, опечатка в ключе не компилируется.
 */
export type OperationId = keyof typeof operations;

/** Входы операции: второй аргумент мутатора orval (`{ headers }`, `{ params }`, `{ data }`). */
export type OperationArgs = Readonly<Record<string, unknown>>;

/** Обработчик заглушки возвращает тело успешного ответа (без обёртки `{ data, status, headers }`). */
export type StubHandler = (args: OperationArgs) => unknown;

type Version = `v${number}.${number}.${number}`;

/** Заглушенные операции и версии, в которых они становятся настоящими (не раньше `x-crane-since`). */
export type StubTable = Partial<Record<OperationId, Version>>;

/** Модули данных заглушек: загрузчики, которые в production без демо отсутствуют вовсе. */
export type StubModules = Readonly<
  Record<string, () => Promise<Readonly<Record<string, unknown>>>>
>;
