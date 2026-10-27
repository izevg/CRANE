// Запуск дочерних процессов без оболочки: аргументы только массивом.
import { spawnSync } from "node:child_process";

export interface Result {
  status: number;
  stdout: string;
  stderr: string;
}

export interface Options {
  cwd?: string;
  /** Добавляется к окружению процесса; сюда кладут секреты, чтобы они не попадали в argv. */
  env?: Record<string, string>;
  input?: string;
  timeoutMs?: number;
}

export const MS_PER_SECOND = 1000;
export const SECONDS_PER_MINUTE = 60;
const BYTES_PER_KIB = 1024;
const BYTES_PER_MIB = BYTES_PER_KIB * BYTES_PER_KIB;
const MAX_BUFFER_MIB = 256;
const DEFAULT_TIMEOUT_MINUTES = 10;
const DEFAULT_TIMEOUT_MS = DEFAULT_TIMEOUT_MINUTES * SECONDS_PER_MINUTE * MS_PER_SECOND;
const MAX_BUFFER_BYTES = MAX_BUFFER_MIB * BYTES_PER_MIB;
const TAIL_CHARS = 2000;

/** Выполняет команду и возвращает результат при любом коде выхода; бросает, только если процесс не запустился. */
export function run(file: string, args: string[], options: Options = {}): Result {
  const result = spawnSync(file, args, {
    ...(options.cwd === undefined ? {} : { cwd: options.cwd }),
    ...(options.input === undefined ? {} : { input: options.input }),
    env: { ...process.env, ...options.env },
    encoding: "utf8",
    maxBuffer: MAX_BUFFER_BYTES,
    timeout: options.timeoutMs ?? DEFAULT_TIMEOUT_MS,
  });
  if (result.error) {
    throw new Error(`${file} ${args.join(" ")}: ${result.error.message}`);
  }
  return { status: result.status ?? -1, stdout: result.stdout, stderr: result.stderr };
}

/** Синхронная пауза: стенд последовательный, асинхронность только усложнила бы очистку. */
export function sleep(ms: number): void {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(Int32Array.BYTES_PER_ELEMENT)), 0, 0, ms);
}

export const tail = (text: string): string => text.trim().slice(-TAIL_CHARS);

/** То же, но ненулевой код выхода — ошибка с хвостом stderr. */
export function runOk(file: string, args: string[], options: Options = {}): string {
  const result = run(file, args, options);
  if (result.status !== 0) {
    throw new Error(
      `${file} ${args.join(" ")} завершилась с кодом ${result.status}\n${tail(result.stderr)}`,
    );
  }
  return result.stdout;
}

export const docker = (args: string[], options: Options = {}): string =>
  runOk("docker", args, options);

export function log(message: string): void {
  process.stdout.write(`${message}\n`);
}

export function warn(message: string): void {
  process.stderr.write(`${message}\n`);
}
