// Отказ зависимости как скоуп: остановить (или заморозить) сервис, выполнить проверку, гарантированно вернуть.
import { parseObjectCount } from "./audit.ts";
import { docker, MS_PER_SECOND, run, sleep, tail, warn } from "./exec.ts";

type Compose = (args: string[]) => string;

export const UP_TIMEOUT_SECONDS = 240;
export const DOWN_TIMEOUT_SECONDS = 5;
const READY_TIMEOUT_SECONDS = 120;
const READY_STEP_MS = 500;

/** Сервисы стенда, которые можно остановить. */
export type Service = "valkey" | "postgres" | "seaweedfs";
/** Остановка контейнера или заморозка процессов (молчащий сервис: соединения открыты, ответов нет). */
export type Outage = "stop" | "pause";

/** `crane healthcheck` в контейнере crane: та же проба, что /healthz/ready (admin-порт наружу не проброшен). */
export const craneReady = (crane: string): boolean =>
  run("docker", ["exec", crane, "crane", "healthcheck"]).status === 0;

/** Цикл пробы с коротким шагом и дедлайном: ожидание готовности, а не пауза наугад. */
function waitReady(ready: () => boolean): void {
  const deadline = Date.now() + READY_TIMEOUT_SECONDS * MS_PER_SECOND;
  while (!ready()) {
    if (Date.now() > deadline) {
      throw new Error(`crane не стал готов за ${READY_TIMEOUT_SECONDS} с после возврата сервиса`);
    }
    sleep(READY_STEP_MS);
  }
}

/** Поднимает остановленный сервис, не трогая соседей и не пересоздавая контейнер, и ждёт его healthcheck. */
export function upService(composeRun: Compose, service: string): void {
  composeRun([
    "up",
    "-d",
    "--wait",
    "--wait-timeout",
    String(UP_TIMEOUT_SECONDS),
    "--no-recreate",
    "--no-deps",
    service,
  ]);
}

const KINDS: Record<
  Outage,
  { down: (service: Service) => string[]; up: (c: Compose, s: Service) => void }
> = {
  stop: {
    down: (service) => ["stop", "--timeout", String(DOWN_TIMEOUT_SECONDS), service],
    up: upService,
  },
  pause: {
    down: (service) => ["pause", service],
    up: (composeRun, service) => {
      composeRun(["unpause", service]);
    },
  },
};

/**
 * Отказ сервиса на время `during`. Сервис возвращается и crane ждётся до готовности и после сбоя `during`,
 * чтобы упавший сценарий не оставил стенд без зависимости. Если `during` уже упал, ошибка восстановления
 * только логируется: первопричина важнее и пробрасывается как есть.
 */
export function withOutage<T>(
  composeRun: Compose,
  ready: () => boolean,
  request: { service: Service; kind: Outage },
  during: () => T,
): T {
  const { service, kind } = request;
  const restore = (): void => {
    KINDS[kind].up(composeRun, service);
    // Не `up --wait crane`: healthcheck compose ходит раз в 10 с, а после долгого отказа контейнер
    // уже помечен unhealthy. Ждём ту же пробу напрямую.
    waitReady(ready);
  };
  let result: T;
  try {
    composeRun(KINDS[kind].down(service));
    result = during();
  } catch (error) {
    try {
      restore();
    } catch (restoreError) {
      warn(`отказ ${service}: восстановление не удалось: ${String(restoreError)}`);
    }
    throw error;
  }
  restore();
  return result;
}

/** Число объектов CAS в бакете: `fs.tree` каталога `blobs/` через `weed shell` внутри контейнера SeaweedFS. */
export function countObjects(seaweedfs: string, bucket: string): number {
  const output = docker(["exec", "-i", seaweedfs, "weed", "shell", "-master=127.0.0.1:9333"], {
    input: `fs.tree /buckets/${bucket}/blobs\n`,
  });
  const count = parseObjectCount(output);
  if (count === undefined) {
    throw new Error(`не разобрать число объектов бакета: ${tail(output)}`);
  }
  return count;
}
