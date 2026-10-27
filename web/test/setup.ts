import "@testing-library/jest-dom/vitest";
import { cleanup, configure } from "@testing-library/react";
import { afterEach } from "vitest";

// Под нагрузкой CI и локальной машины секунда по умолчанию на findBy/waitFor флакает.
configure({ asyncUtilTimeout: 10_000 });

// jsdom не реализует часть браузерных API, на которые опираются Base UI и sonner.
// Заглушки нейтральные: поведение конкретного теста задаёт сам тест (см. ~test/matchMedia).
if (typeof window.matchMedia !== "function") {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: (query: string) => ({
      matches: false,
      media: query,
      addEventListener: () => undefined,
      removeEventListener: () => undefined,
    }),
  });
}

class ResizeObserverStub {
  observe(): void {}
  unobserve(): void {}
  disconnect(): void {}
}
globalThis.ResizeObserver ??= ResizeObserverStub;

if (typeof globalThis.PointerEvent === "undefined") {
  // user-event шлёт pointer-события; без конструктора Base UI теряет координаты и тип указателя.
  class PointerEventStub extends MouseEvent {
    readonly pointerId: number;
    readonly pointerType: string;
    constructor(type: string, init: PointerEventInit = {}) {
      super(type, init);
      this.pointerId = init.pointerId ?? 1;
      this.pointerType = init.pointerType ?? "mouse";
    }
  }
  globalThis.PointerEvent = PointerEventStub as unknown as typeof PointerEvent;
}

Element.prototype.scrollIntoView ??= () => undefined;
Element.prototype.setPointerCapture ??= () => undefined;
Element.prototype.releasePointerCapture ??= () => undefined;
Element.prototype.hasPointerCapture ??= () => false;
document.elementFromPoint ??= () => null;

afterEach(() => {
  cleanup();
});
