import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { announce } from "~/shared/lib/a11y";
import { focusHeadingWhenReady, shouldMoveFocus } from "./routeFocus";

vi.mock("~/shared/lib/a11y", () => ({ announce: vi.fn() }));

// больше таймаута ожидания заголовка (1 с) и меньше его, соответственно
const TIMEOUT_MS = 1100;
const SHORT_MS = 300;

describe("shouldMoveFocus", () => {
  it("смена pathname после первой загрузки — переносим", () => {
    // Arrange
    const info = { fromLocation: {}, pathChanged: true };

    // Act
    const move = shouldMoveFocus(info);

    // Assert
    expect(move).toBe(true);
  });

  it("смена только search/hash (pathname тот же) — не трогаем", () => {
    // Arrange
    const info = { fromLocation: {}, pathChanged: false };

    // Act
    const move = shouldMoveFocus(info);

    // Assert
    expect(move).toBe(false);
  });

  it("первая загрузка документа (нет прежнего положения) — не трогаем", () => {
    // Arrange
    const info = { pathChanged: true };

    // Act
    const move = shouldMoveFocus(info);

    // Assert
    expect(move).toBe(false);
  });
});

describe("focusHeadingWhenReady", () => {
  let root: HTMLElement;
  const pending: (() => void)[] = [];

  // Ожидания, не дошедшие до конца в своём тесте, снимаются: иначе их наблюдатели видят чужие правки DOM
  function start(stale: HTMLElement | null): () => void {
    const cancel = focusHeadingWhenReady(document.body, stale);
    pending.push(cancel);
    return cancel;
  }

  beforeEach(() => {
    vi.mocked(announce).mockClear();
    vi.useFakeTimers();
    root = document.createElement("main");
    document.body.append(root);
  });

  afterEach(() => {
    for (const cancel of pending.splice(0)) {
      cancel();
    }
    root.remove();
    vi.useRealTimers();
  });

  it("фокусирует уже стоящий h1 и ставит ему tabindex=-1", () => {
    // Arrange
    root.innerHTML = "<h1>Статус</h1>";

    // Act
    start(null);

    // Assert
    const heading = root.querySelector("h1");
    expect(heading).toHaveFocus();
    expect(heading).toHaveAttribute("tabindex", "-1");
  });

  it("ждёт h1 новой страницы и не фокусирует прежний, пока он не заменён", async () => {
    // Arrange
    root.innerHTML = "<h1>Старая</h1>";
    const stale = root.querySelector("h1");
    start(stale);
    root.innerHTML = "<h1>Новая</h1>";

    // Act
    await vi.advanceTimersByTimeAsync(0);

    // Assert
    expect(root.querySelector("h1")).toHaveFocus();
    expect(root).toHaveTextContent("Новая");
  });

  it("нет h1 к таймауту — объявляется заголовок документа вместо фокуса", async () => {
    // Arrange
    document.title = "Пустая — CRANE";
    start(null);

    // Act
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);

    // Assert
    expect(announce).toHaveBeenCalledWith("Пустая — CRANE");
  });

  it("отмена снимает ожидание: h1, появившийся позже, фокус не получает", async () => {
    // Arrange
    const cancel = start(null);
    cancel();
    root.innerHTML = "<h1>Поздний</h1>";

    // Act
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);

    // Assert
    expect(root.querySelector("h1")).not.toHaveFocus();
  });

  it("скелетон без h1 фокус не получает", async () => {
    // Arrange
    root.innerHTML = "<div aria-busy='true'><div></div></div>";
    start(null);

    // Act
    await vi.advanceTimersByTimeAsync(SHORT_MS);

    // Assert
    expect(document.activeElement).toBe(document.body);
  });

  it("фокус без прокрутки: страница уже на нужном месте", () => {
    // Arrange
    root.innerHTML = "<h1>Статус</h1>";
    const focus = vi.spyOn(HTMLElement.prototype, "focus");

    // Act
    start(null);

    // Assert
    expect(focus).toHaveBeenCalledWith({ preventScroll: true });
    focus.mockRestore();
  });

  it("собственный tabindex заголовка не перезаписывается", () => {
    // Arrange
    root.innerHTML = '<h1 tabindex="0">Статус</h1>';

    // Act
    start(null);

    // Assert
    expect(root.querySelector("h1")).toHaveAttribute("tabindex", "0");
  });

  it("после фокуса ожидание снято: позднее объявление и новый h1 ничего не двигают", async () => {
    // Arrange
    root.innerHTML = "<h1>Первый</h1>";
    start(null);
    const first = root.querySelector("h1");
    root.insertAdjacentHTML("beforeend", "<h1>Второй</h1>");

    // Act
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);

    // Assert
    expect(first).toHaveFocus();
    expect(announce).not.toHaveBeenCalled();
  });

  it("к таймауту прежний заголовок ещё на месте — фокус не на нём, объявляется заголовок документа", async () => {
    // Arrange
    root.innerHTML = "<h1>Тот же</h1>";
    document.title = "Страница — CRANE";
    const same = root.querySelector("h1");
    start(same);

    // Act
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);

    // Assert
    expect(same).not.toHaveFocus();
    expect(announce).toHaveBeenCalledWith("Страница — CRANE");
  });

  it("h1 вне main фокус не получает", async () => {
    // Arrange
    document.body.insertAdjacentHTML("beforeend", '<div id="outside"><h1>Не в main</h1></div>');
    start(null);

    // Act
    await vi.advanceTimersByTimeAsync(SHORT_MS);

    // Assert
    expect(document.querySelector("#outside h1")).not.toHaveFocus();
    document.querySelector("#outside")?.remove();
  });

  it("после фокуса наблюдатель отключён, а таймер ожидания не срабатывает: утечек нет", async () => {
    // Arrange
    root.innerHTML = "<h1>Статус</h1>";
    const disconnect = vi.spyOn(MutationObserver.prototype, "disconnect");
    const focus = vi.spyOn(HTMLElement.prototype, "focus");
    start(null);

    // Act
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);

    // Assert
    expect(disconnect).toHaveBeenCalledTimes(1);
    expect(focus).toHaveBeenCalledTimes(1);
    expect(announce).not.toHaveBeenCalled();
    disconnect.mockRestore();
    focus.mockRestore();
  });

  it("h1, появившийся после таймаута, фокус не перехватывает: ожидание снято", async () => {
    // Arrange
    start(null);
    await vi.advanceTimersByTimeAsync(TIMEOUT_MS);
    root.innerHTML = "<h1>Слишком поздно</h1>";

    // Act
    await vi.advanceTimersByTimeAsync(SHORT_MS);

    // Assert
    expect(root.querySelector("h1")).not.toHaveFocus();
  });
});
