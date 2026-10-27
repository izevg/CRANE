import { vi } from "vitest";

/** Ломает `localStorage` и `sessionStorage`: любой доступ бросает (как в песочнице с запретом cookies). */
export function brokenStorage(): void {
  const fail = (): never => {
    throw new Error("storage недоступен");
  };
  vi.spyOn(Storage.prototype, "getItem").mockImplementation(fail);
  vi.spyOn(Storage.prototype, "setItem").mockImplementation(fail);
  vi.spyOn(Storage.prototype, "removeItem").mockImplementation(fail);
}
