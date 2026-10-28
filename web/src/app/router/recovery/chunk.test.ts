import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { chunkAction, decideChunkAction, markReload, RELOAD_WINDOW_MS } from "./chunk";

const NOW = 1_000_000;

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
