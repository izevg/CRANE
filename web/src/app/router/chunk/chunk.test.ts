import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  ChunkLoadError,
  chunkAction,
  decideChunkAction,
  isChunkLoadError,
  lazyPage,
  markReload,
  RELOAD_WINDOW_MS,
} from "./chunk";

const NOW = 1_000_000;

describe("isChunkLoadError", () => {
  it.each([
    "Failed to fetch dynamically imported module: https://crane.example/assets/Page-ab12.js",
    "error loading dynamically imported module: https://crane.example/assets/Page-ab12.js",
    "Importing a module script failed.",
    "Unable to preload CSS for /assets/Page-ab12.css",
  ])("браузерное сообщение %j — сбой чанка", (message) => {
    // Arrange
    const error = new TypeError(message);

    // Act
    const chunk = isChunkLoadError(error);

    // Assert
    expect(chunk).toBe(true);
  });

  it("обычная ошибка и не-Error — не сбой чанка", () => {
    // Arrange
    const errors = [
      new Error("Cannot read properties of undefined"),
      "Failed to fetch dynamically imported module",
    ];

    // Act
    const flags = errors.map(isChunkLoadError);

    // Assert
    expect(flags).toEqual([false, false]);
  });
});

describe("lazyPage", () => {
  it("сбой загрузки чанка становится ChunkLoadError с исходной причиной", async () => {
    // Arrange
    const cause = new TypeError("Failed to fetch dynamically imported module: x");
    const Page = lazyPage(() => Promise.reject(cause));
    await Page.preload?.();

    // Act
    const failure = () => Page({});

    // Assert
    expect(failure).toThrow(ChunkLoadError);
    expect(failure).toThrow(expect.objectContaining({ name: "ChunkLoadError", cause }));
  });

  it("другая ошибка импорта остаётся собой", async () => {
    // Arrange
    const bug = new SyntaxError("Unexpected token");
    const Page = lazyPage(() => Promise.reject(bug));
    await Page.preload?.();

    // Act
    const failure = () => Page({});

    // Assert
    expect(failure).toThrow(bug);
  });

  it("сообщение ChunkLoadError не похоже на «модуль не найден»: роутер не перезагрузит страницу сам", () => {
    // Arrange
    const error = new ChunkLoadError(new Error("x"));

    // Act
    const looksLikeMissingModule = isChunkLoadError(error);

    // Assert
    expect(looksLikeMissingModule).toBe(false);
  });
});

describe("chunkAction", () => {
  it("нет сети — offline, перезагрузка не поможет", () => {
    // Arrange
    const state = { online: false, lastReloadAt: undefined, now: NOW };

    // Act
    const action = chunkAction(state);

    // Assert
    expect(action).toBe("offline");
  });

  it("сеть есть, перезагрузок не было — одна перезагрузка", () => {
    // Arrange
    const state = { online: true, lastReloadAt: undefined, now: NOW };

    // Act
    const action = chunkAction(state);

    // Assert
    expect(action).toBe("reload");
  });

  it("повторный сбой в окне после перезагрузки — экран ошибки, без цикла", () => {
    // Arrange
    const state = { online: true, lastReloadAt: NOW - RELOAD_WINDOW_MS + 1, now: NOW };

    // Act
    const action = chunkAction(state);

    // Assert
    expect(action).toBe("show");
  });

  it("сбой после окна — снова одна перезагрузка (новый деплой)", () => {
    // Arrange
    const state = { online: true, lastReloadAt: NOW - RELOAD_WINDOW_MS, now: NOW };

    // Act
    const action = chunkAction(state);

    // Assert
    expect(action).toBe("reload");
  });

  it("офлайн важнее недавней перезагрузки", () => {
    // Arrange
    const state = { online: false, lastReloadAt: NOW, now: NOW };

    // Act
    const action = chunkAction(state);

    // Assert
    expect(action).toBe("offline");
  });
});

describe("decideChunkAction: метка в sessionStorage", () => {
  beforeEach(() => {
    sessionStorage.clear();
    vi.spyOn(navigator, "onLine", "get").mockReturnValue(true);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("без метки — reload; после markReload — show", () => {
    // Arrange
    const before = decideChunkAction();
    markReload();

    // Act
    const after = decideChunkAction();

    // Assert
    expect(before).toBe("reload");
    expect(after).toBe("show");
  });

  it("мусор вместо метки игнорируется", () => {
    // Arrange
    sessionStorage.setItem("crane.chunk-reload-at", "не число");

    // Act
    const action = decideChunkAction();

    // Assert
    expect(action).toBe("reload");
  });

  it("без сети — offline", () => {
    // Arrange
    vi.spyOn(navigator, "onLine", "get").mockReturnValue(false);

    // Act
    const action = decideChunkAction();

    // Assert
    expect(action).toBe("offline");
  });
});
