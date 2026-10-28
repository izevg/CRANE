import { MutationObserver, QueryObserver } from "@tanstack/react-query";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiProblem } from "~/shared/api";
import { createQueryClient } from "./queryClient";
import { sessionEnded } from "./sessionEnded";

const STATUS = { unauthorized: 401, notFound: 404, serverError: 500 } as const;
const SESSION_ENDED = new ApiProblem({ kind: "unauthenticated", status: STATUS.unauthorized });

function rejecting(error: unknown) {
  return () => Promise.reject(error);
}

beforeEach(() => {
  sessionEnded.set(false);
});

describe("createQueryClient: завершение сессии", () => {
  it("401 запроса поднимает флаг для баннера", async () => {
    // Arrange
    const client = createQueryClient();

    // Act
    await client
      .fetchQuery({ queryKey: ["me"], queryFn: rejecting(SESSION_ENDED) })
      .catch(() => undefined);

    // Assert
    expect(sessionEnded.get()).toBe(true);
  });

  it("401 мутации тоже поднимает флаг", async () => {
    // Arrange
    const client = createQueryClient();
    const mutation = new MutationObserver(client, { mutationFn: rejecting(SESSION_ENDED) });

    // Act
    await mutation.mutate(undefined).catch(() => undefined);

    // Assert
    expect(sessionEnded.get()).toBe(true);
  });

  it("step-up глобально не обрабатывается", async () => {
    // Arrange
    const client = createQueryClient();
    const stepUp = new ApiProblem({
      kind: "step-up",
      status: STATUS.unauthorized,
      requiredAcr: "mfa",
    });

    // Act
    await client
      .fetchQuery({ queryKey: ["x"], queryFn: rejecting(stepUp), retry: false })
      .catch(() => undefined);

    // Assert
    expect(sessionEnded.get()).toBe(false);
  });

  it("прочие ошибки (500) завершением сессии не считаются", async () => {
    // Arrange
    const client = createQueryClient();
    const failure = new ApiProblem({ kind: "problem", status: STATUS.serverError });

    // Act
    await client
      .fetchQuery({ queryKey: ["x"], queryFn: rejecting(failure), retry: false })
      .catch(() => undefined);

    // Assert
    expect(sessionEnded.get()).toBe(false);
  });

  it("ошибка не из API завершением сессии не считается", async () => {
    // Arrange
    const client = createQueryClient();

    // Act
    await client
      .fetchQuery({ queryKey: ["x"], queryFn: rejecting(new TypeError("баг")), retry: false })
      .catch(() => undefined);

    // Assert
    expect(sessionEnded.get()).toBe(false);
  });
});

describe("createQueryClient: после мутации", () => {
  it("успешная мутация перечитывает активные запросы", async () => {
    // Arrange
    const client = createQueryClient();
    const queryFn = vi.fn().mockResolvedValue("данные");
    new QueryObserver(client, { queryKey: ["status"], queryFn }).subscribe(() => undefined);
    await vi.waitFor(() => expect(queryFn).toHaveBeenCalledTimes(1));
    const mutation = new MutationObserver(client, { mutationFn: () => Promise.resolve("ok") });

    // Act
    await mutation.mutate(undefined);

    // Assert
    expect(queryFn).toHaveBeenCalledTimes(2);
  });

  it("неудачная мутация запросы не трогает", async () => {
    // Arrange
    const client = createQueryClient();
    const queryFn = vi.fn().mockResolvedValue("данные");
    new QueryObserver(client, { queryKey: ["status"], queryFn }).subscribe(() => undefined);
    await vi.waitFor(() => expect(queryFn).toHaveBeenCalledTimes(1));
    const mutation = new MutationObserver(client, {
      mutationFn: rejecting(new ApiProblem({ kind: "problem", status: STATUS.notFound })),
    });

    // Act
    await mutation.mutate(undefined).catch(() => undefined);

    // Assert
    expect(queryFn).toHaveBeenCalledTimes(1);
  });
});

describe("createQueryClient: повторы", () => {
  it("запрос с 4xx не повторяется (политика retry подключена)", async () => {
    // Arrange
    const client = createQueryClient();
    const queryFn = vi
      .fn()
      .mockRejectedValue(new ApiProblem({ kind: "problem", status: STATUS.notFound }));

    // Act
    await client.fetchQuery({ queryKey: ["x"], queryFn }).catch(() => undefined);

    // Assert
    expect(queryFn).toHaveBeenCalledTimes(1);
  });

  it("свежесть: только что загруженное не перечитывается сразу", async () => {
    // Arrange
    const client = createQueryClient();
    const queryFn = vi.fn().mockResolvedValue("данные");
    await client.fetchQuery({ queryKey: ["fresh"], queryFn });

    // Act
    await client.fetchQuery({ queryKey: ["fresh"], queryFn });

    // Assert
    expect(queryFn).toHaveBeenCalledTimes(1);
  });
});
