import { useSyncExternalStore } from "react";

/** Булев флаг в памяти с подпиской: один вид хранилища для всех «включено/выключено» состояний приложения. */
export type Flag = {
  get: () => boolean;
  set: (value: boolean) => void;
  subscribe: (listener: () => void) => () => void;
  /** Хук: компонент перерисовывается при изменении флага. */
  use: () => boolean;
};

export function createFlag(initial = false): Flag {
  let value = initial;
  const listeners = new Set<() => void>();
  const get = (): boolean => value;
  const subscribe = (listener: () => void): (() => void) => {
    listeners.add(listener);
    return () => {
      listeners.delete(listener);
    };
  };
  return {
    get,
    subscribe,
    set(next) {
      if (value === next) {
        return;
      }
      value = next;
      for (const listener of listeners) {
        listener();
      }
    },
    use: () => useSyncExternalStore(subscribe, get, get),
  };
}
