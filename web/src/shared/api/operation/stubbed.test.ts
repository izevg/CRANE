import { describe, expect, it, vi } from "vitest";
import { ApiProblem } from "../http/problem";
import type { StubEntries } from "../stubs/table/types";
import { type StubDeps, stubbedFn } from "./stubbed";

const BODY = { version: "demo" };

function deps(
  overrides: Partial<StubDeps> & { load?: () => Promise<Record<string, unknown>> } = {},
) {
  const { load, ...rest } = overrides;
  const entries: StubEntries = {
    getStatus: { replacedIn: "v0.4.0", load },
  };
  return { demo: true, entries, overrideFor: () => undefined, ...rest } satisfies StubDeps;
}

const args = { headers: { a: 1 } };
const same = () => args;

describe("stubbedFn: демо выключено", () => {
  it("заглушенная операция бросает planned с версией замены", async () => {
    // Arrange
    const original = vi.fn();
    const load = vi.fn();
    const run = stubbedFn(original, "getStatus", same, deps({ demo: false, load }));

    // Act
    const error = await run(undefined).catch((cause: unknown) => cause);

    // Assert
    expect(error).toBeInstanceOf(ApiProblem);
    expect(error).toMatchObject({ kind: "planned", replacedIn: "v0.4.0" });
  });

  it("не идёт в сеть и не грузит данные заглушки", async () => {
    // Arrange
    const original = vi.fn();
    const load = vi.fn();
    const run = stubbedFn(original, "getStatus", same, deps({ demo: false, load }));

    // Act
    await run(undefined).catch(() => undefined);

    // Assert
    expect(original).not.toHaveBeenCalled();
    expect(load).not.toHaveBeenCalled();
  });

  it("запись без загрузчика (production-сборка) тоже planned", async () => {
    // Arrange
    const original = vi.fn();
    const run = stubbedFn(original, "getStatus", same, deps({ demo: true }));

    // Act
    const error = await run(undefined).catch((cause: unknown) => cause);

    // Assert
    expect(error).toMatchObject({ kind: "planned" });
  });
});

describe("stubbedFn: демо включено", () => {
  it("отдаёт данные заглушки в форме ответа транспорта и не идёт в сеть", async () => {
    // Arrange
    const original = vi.fn();
    const getStatus = vi.fn().mockReturnValue(BODY);
    const run = stubbedFn(
      original,
      "getStatus",
      same,
      deps({ load: () => Promise.resolve({ getStatus }) }),
    );

    // Act
    const result = await run(undefined);

    // Assert
    expect(result).toMatchObject({ data: BODY, status: 200 });
    expect(original).not.toHaveBeenCalled();
  });

  it("обработчик получает входы операции", async () => {
    // Arrange
    const getStatus = vi.fn().mockReturnValue(BODY);
    const run = stubbedFn(
      vi.fn(),
      "getStatus",
      same,
      deps({ load: () => Promise.resolve({ getStatus }) }),
    );

    // Act
    await run(undefined);

    // Assert
    expect(getStatus).toHaveBeenCalledWith(args);
  });

  it("модуль без обработчика — ошибка разработчика, а не пустой ответ", async () => {
    // Arrange
    const run = stubbedFn(vi.fn(), "getStatus", same, deps({ load: () => Promise.resolve({}) }));

    // Act
    const error = await run(undefined).catch((cause: unknown) => cause);

    // Assert
    expect(error).toBeInstanceOf(Error);
    expect(error).not.toBeInstanceOf(ApiProblem);
    expect((error as Error).message).toContain("getStatus");
  });
});

describe("stubbedFn: операции вне таблицы", () => {
  it("идут в сеть, заглушка не трогается", async () => {
    // Arrange
    const original = vi.fn().mockResolvedValue("сеть");
    const load = vi.fn();
    const run = stubbedFn(original, "listTokens", same, deps({ load }));

    // Act
    const result = await run("ctx");

    // Assert
    expect(result).toBe("сеть");
    expect(original).toHaveBeenCalledWith("ctx");
    expect(load).not.toHaveBeenCalled();
  });
});

describe("stubbedFn: подмена Storybook", () => {
  it("подмена сильнее таблицы и работает при выключенном демо", async () => {
    // Arrange
    const original = vi.fn();
    const forced = vi.fn().mockReturnValue({ version: "из истории" });
    const run = stubbedFn(
      original,
      "getStatus",
      same,
      deps({ demo: false, overrideFor: () => forced }),
    );

    // Act
    const result = await run(undefined);

    // Assert
    expect(result).toMatchObject({ data: { version: "из истории" } });
    expect(original).not.toHaveBeenCalled();
  });

  it("подмена может бросить ApiProblem — состояние ошибки", async () => {
    // Arrange
    const failure = new ApiProblem({ kind: "problem", status: 500 });
    const run = stubbedFn(
      vi.fn(),
      "getStatus",
      same,
      deps({
        overrideFor: () => () => {
          throw failure;
        },
      }),
    );

    // Act
    const error = await run(undefined).catch((cause: unknown) => cause);

    // Assert
    expect(error).toBe(failure);
  });

  it("подмена операции вне таблицы тоже применяется", async () => {
    // Arrange
    const original = vi.fn();
    const run = stubbedFn(
      original,
      "listTokens",
      same,
      deps({ overrideFor: (id) => (id === "listTokens" ? () => ({ items: [] }) : undefined) }),
    );

    // Act
    const result = await run(undefined);

    // Assert
    expect(result).toMatchObject({ data: { items: [] } });
  });
});
