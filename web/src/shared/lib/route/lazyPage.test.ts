import { describe, expect, it } from "vitest";
import { ChunkLoadError, isChunkLoadError, lazyPage } from "./lazyPage";

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
    const Page = lazyPage(() => Promise.reject<Record<string, () => null>>(cause), "Page");
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
    const Page = lazyPage(() => Promise.reject<Record<string, () => null>>(bug), "Page");
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
