import { describe, expect, it, vi } from "vitest";
import { ApiProblem } from "../http/problem";
import type { StubHandler, StubModules } from "../stubs/table/types";
import { type StubDeps, stubbedFn } from "./stubbed";

const BODY = { version: "demo" };
const args = { headers: { a: 1 } };
const same = () => args;

function deps(overrides: Partial<StubDeps> = {}): StubDeps {
  return {
    demo: true,
    versions: { getStatus: "v0.4.0" },
    modules: {},
    overrideFor: () => undefined,
    ...overrides,
  };
}

function modulesWith(handler: StubHandler): StubModules {
  return { status: () => Promise.resolve({ getStatus: handler }) };
}

describe("stubbedFn: демо выключено", () => {
  it("заглушенная операция бросает planned с версией замены", async () => {
    // Arrange
    const run = stubbedFn(vi.fn(), "getStatus", same, deps({ demo: false }));

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
    const run = stubbedFn(
      original,
      "getStatus",
      same,
      deps({ demo: false, modules: { status: load } }),
    );

    // Act
    await run(undefined).catch(() => undefined);

    // Assert
    expect(original).not.toHaveBeenCalled();
    expect(load).not.toHaveBeenCalled();
  });
});

describe("stubbedFn: демо включено", () => {
  it("отдаёт данные заглушки в форме ответа транспорта и не идёт в сеть", async () => {
    // Arrange
    const original = vi.fn();
    const handler = vi.fn().mockReturnValue(BODY);
    const run = stubbedFn(original, "getStatus", same, deps({ modules: modulesWith(handler) }));

    // Act
    const result = await run(undefined);

    // Assert
    expect(result).toMatchObject({ data: BODY, status: 200 });
    expect(original).not.toHaveBeenCalled();
  });

  it("обработчик получает входы операции", async () => {
    // Arrange
    const handler = vi.fn().mockReturnValue(BODY);
    const run = stubbedFn(vi.fn(), "getStatus", same, deps({ modules: modulesWith(handler) }));

    // Act
    await run(undefined);

    // Assert
    expect(handler).toHaveBeenCalledWith(args);
  });

  it("обработчик ищется среди всех модулей по имени операции", async () => {
    // Arrange
    const handler = vi.fn().mockReturnValue(BODY);
    const modules: StubModules = {
      me: () => Promise.resolve({ getMe: () => "не тот" }),
      status: () => Promise.resolve({ getStatus: handler }),
    };
    const run = stubbedFn(vi.fn(), "getStatus", same, deps({ modules }));

    // Act
    const result = await run(undefined);

    // Assert
    expect(result).toMatchObject({ data: BODY });
  });

  it("модули данных грузятся один раз на все вызовы", async () => {
    // Arrange
    const load = vi.fn().mockResolvedValue({ getStatus: () => BODY });
    const run = stubbedFn(vi.fn(), "getStatus", same, deps({ modules: { status: load } }));
    await run(undefined);

    // Act
    await run(undefined);

    // Assert
    expect(load).toHaveBeenCalledTimes(1);
  });

  it("нет обработчика ни в одном модуле — ошибка разработчика с именем операции, не пустой ответ", async () => {
    // Arrange
    const run = stubbedFn(
      vi.fn(),
      "getStatus",
      same,
      deps({ modules: { me: () => Promise.resolve({}) } }),
    );

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
    const run = stubbedFn(original, "listTokens", same, deps({ modules: { status: load } }));

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
    const run = stubbedFn(
      vi.fn(),
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
