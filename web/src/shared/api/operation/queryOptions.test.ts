import { afterEach, describe, expect, it, vi } from "vitest";
import { getGetMeQueryOptions, getGetStatusQueryOptions } from "../generated/admin.gen";
import { override, resetOverrides } from "../stubs/override/override";
import { withMutation } from "./mutationOptions";
import { withOperation } from "./queryOptions";

const META = { operationId: "listTokens", operationName: "listTokens" } as const;
const CONTEXT = { signal: new AbortController().signal, queryKey: ["x"] };

afterEach(() => {
  resetOverrides();
});

describe("withOperation: сборка опций", () => {
  it("getStatus из сгенерированных опций отдаёт заглушку и опрашивается", async () => {
    // Arrange
    const options = getGetStatusQueryOptions();

    // Act
    const result = await (options.queryFn as (ctx: unknown) => Promise<unknown>)(CONTEXT);

    // Assert
    expect(result).toMatchObject({ status: 200, data: { version: "0.3.0" } });
    expect(options.refetchInterval).toBeTypeOf("function");
  });

  it("getMe отдаёт профиль заглушки и не опрашивается", async () => {
    // Arrange
    const options = getGetMeQueryOptions();

    // Act
    const result = await (options.queryFn as (ctx: unknown) => Promise<unknown>)(CONTEXT);

    // Assert
    expect(result).toMatchObject({ data: { principal: { display: "Иванова А." } } });
    expect(options.refetchInterval).toBeUndefined();
  });

  it("опции без queryFn возвращаются как есть", () => {
    // Arrange
    const options = { queryKey: ["x"], queryFn: undefined };

    // Act
    const result = withOperation(options, {}, META);

    // Assert
    expect(result).toBe(options);
  });
});

describe("withOperation: входы заглушки", () => {
  it("подмена получает параметры операции, с которыми собраны опции", async () => {
    // Arrange
    const handler = vi.fn().mockReturnValue({ items: [] });
    override("listTokens", handler);
    const args = { params: { owner: "me" } };
    const options = withOperation({ queryFn: vi.fn() }, args, META);

    // Act
    await (options.queryFn as (ctx: unknown) => Promise<unknown>)(CONTEXT);

    // Assert
    expect(handler).toHaveBeenCalledWith(args);
  });

  it("queryFn, которая не функция (skipToken), остаётся как есть", () => {
    // Arrange
    const options = { queryFn: Symbol("skip") };

    // Act
    const result = withOperation(options, {}, META);

    // Assert
    expect(result.queryFn).toBe(options.queryFn);
  });
});

describe("withMutation", () => {
  it("мутация вне таблицы идёт в исходную mutationFn с теми же переменными", async () => {
    // Arrange
    const calls: unknown[] = [];
    const options = withMutation(
      { mutationFn: (variables: unknown) => Promise.resolve(calls.push(variables)) },
      { url: "/tokens" },
      META,
    );
    const run = options.mutationFn as (variables: unknown) => Promise<unknown>;

    // Act
    await run({ data: 1 });

    // Assert
    expect(calls).toEqual([{ data: 1 }]);
  });

  it("опции без mutationFn возвращаются как есть", () => {
    // Arrange
    const options = {};

    // Act
    const result = withMutation(options, { url: "/x" }, META);

    // Assert
    expect(result).toBe(options);
  });

  it("подмена мутации получает переменные вызова", async () => {
    // Arrange
    const handler = vi.fn().mockReturnValue({ created: true });
    override("createToken", handler);
    const options = withMutation({ mutationFn: vi.fn() }, { url: "/tokens" }, {
      operationId: "createToken",
      operationName: "createToken",
    } as const);
    const run = options.mutationFn as (variables: unknown) => Promise<unknown>;

    // Act
    const result = await run({ data: { name: "ci" } });

    // Assert
    expect(handler).toHaveBeenCalledWith({ data: { name: "ci" } });
    expect(result).toMatchObject({ data: { created: true } });
  });

  it("переменные не объект — подмена получает пустой набор входов", async () => {
    // Arrange
    const handler = vi.fn().mockReturnValue({});
    override("revokeToken", handler);
    const options = withMutation({ mutationFn: vi.fn() }, { url: "/tokens/x" }, {
      operationId: "revokeToken",
      operationName: "revokeToken",
    } as const);
    const run = options.mutationFn as (variables: unknown) => Promise<unknown>;

    // Act
    await run(undefined);

    // Assert
    expect(handler).toHaveBeenCalledWith({});
  });

  it("переменные null — подмена получает пустой набор входов", async () => {
    // Arrange
    const handler = vi.fn().mockReturnValue({});
    override("revokeToken", handler);
    const options = withMutation({ mutationFn: vi.fn() }, { url: "/tokens/x" }, {
      operationId: "revokeToken",
      operationName: "revokeToken",
    } as const);
    const run = options.mutationFn as (variables: unknown) => Promise<unknown>;

    // Act
    await run(null);

    // Assert
    expect(handler).toHaveBeenCalledWith({});
  });

  it("mutationFn, которая не функция, остаётся как есть", () => {
    // Arrange
    const options = { mutationFn: "не функция" };

    // Act
    const result = withMutation(options, { url: "/x" }, META);

    // Assert
    expect(result).toBe(options);
  });
});
