import { describe, expect, it, vi } from "vitest";
import { createCsrfSource } from "./csrf";
import { ApiProblem } from "./problem";

describe("createCsrfSource", () => {
  it("читает токен один раз и отдаёт тот же промис всем вызывающим", async () => {
    // Arrange
    const read = vi.fn().mockResolvedValue("tok-1");
    const source = createCsrfSource(read);

    // Act
    const tokens = await Promise.all([source.token(), source.token()]);

    // Assert
    expect(tokens).toEqual(["tok-1", "tok-1"]);
    expect(read).toHaveBeenCalledTimes(1);
  });

  it("после reset перечитывает токен", async () => {
    // Arrange
    const read = vi.fn().mockResolvedValueOnce("old").mockResolvedValueOnce("new");
    const source = createCsrfSource(read);
    await source.token();
    source.reset();

    // Act
    const token = await source.token();

    // Assert
    expect(token).toBe("new");
  });

  it("сбой чтения не запоминается: следующий вызов пробует снова", async () => {
    // Arrange
    const read = vi.fn().mockRejectedValueOnce(new Error("сеть")).mockResolvedValueOnce("tok");
    const source = createCsrfSource(read);
    await source.token().catch(() => undefined);

    // Act
    const token = await source.token();

    // Assert
    expect(token).toBe("tok");
  });

  it("нет сессии (токен null) — unauthenticated без мутации", async () => {
    // Arrange
    const source = createCsrfSource(() => Promise.resolve(null));

    // Act
    const error = await source.token().catch((cause: unknown) => cause);

    // Assert
    expect(error).toBeInstanceOf(ApiProblem);
    expect(error).toMatchObject({ kind: "unauthenticated" });
  });
});
