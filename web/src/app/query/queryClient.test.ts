import { MutationObserver, QueryObserver } from "@tanstack/react-query";
import { describe, expect, it, vi } from "vitest";
import { ApiProblem } from "~/shared/api";
import { createQueryClient } from "./queryClient";

const SESSION_ENDED = new ApiProblem({
  kind: "unauthenticated",
  status: 401,
  reason: "session_expired",
});

function rejecting(error: unknown) {
  return () => Promise.reject(error);
}

describe("createQueryClient: завершение сессии", () => {
  it("401 запроса без наблюдателей (loader при переходе) — переход, не фон", async () => {
    // Arrange
    const onSessionEnded = vi.fn();
    const client = createQueryClient(onSessionEnded);

    // Act
    await client
      .fetchQuery({ queryKey: ["me"], queryFn: rejecting(SESSION_ENDED) })
      .catch(() => undefined);

    // Assert
    expect(onSessionEnded).toHaveBeenCalledWith({ background: false, reason: "session_expired" });
  });

  it("401 запроса с наблюдателем (опрос на экране) — фон", async () => {
    // Arrange
    const onSessionEnded = vi.fn();
    const client = createQueryClient(onSessionEnded);
    const observer = new QueryObserver(client, {
      queryKey: ["status"],
      queryFn: rejecting(SESSION_ENDED),
      retry: false,
    });
    observer.subscribe(() => undefined);

    // Act
    await vi.waitFor(() => expect(onSessionEnded).toHaveBeenCalled());

    // Assert
    expect(onSessionEnded).toHaveBeenCalledWith({ background: true, reason: "session_expired" });
  });

  it("401 мутации — фон: страницу с формой не покидаем", async () => {
    // Arrange
    const onSessionEnded = vi.fn();
    const client = createQueryClient(onSessionEnded);
    const mutation = new MutationObserver(client, { mutationFn: rejecting(SESSION_ENDED) });

    // Act
    await mutation.mutate(undefined).catch(() => undefined);

    // Assert
    expect(onSessionEnded).toHaveBeenCalledWith({ background: true, reason: "session_expired" });
  });

  it("401 без причины — событие без ключа reason", async () => {
    // Arrange
    const onSessionEnded = vi.fn();
    const client = createQueryClient(onSessionEnded);
    const bare = new ApiProblem({ kind: "unauthenticated", status: 401 });

    // Act
    await client.fetchQuery({ queryKey: ["x"], queryFn: rejecting(bare) }).catch(() => undefined);

    // Assert
    expect(onSessionEnded.mock.calls[0]?.[0]).toStrictEqual({ background: false });
  });

  it("step-up глобально не обрабатывается", async () => {
    // Arrange
    const onSessionEnded = vi.fn();
    const client = createQueryClient(onSessionEnded);
    const stepUp = new ApiProblem({ kind: "step-up", status: 401, requiredAcr: "mfa" });

    // Act
    await client
      .fetchQuery({ queryKey: ["x"], queryFn: rejecting(stepUp), retry: false })
      .catch(() => undefined);

    // Assert
    expect(onSessionEnded).not.toHaveBeenCalled();
  });

  it("прочие ошибки (500) завершением сессии не считаются", async () => {
    // Arrange
    const onSessionEnded = vi.fn();
    const client = createQueryClient(onSessionEnded);
    const failure = new ApiProblem({ kind: "problem", status: 500 });

    // Act
    await client
      .fetchQuery({ queryKey: ["x"], queryFn: rejecting(failure), retry: false })
      .catch(() => undefined);

    // Assert
    expect(onSessionEnded).not.toHaveBeenCalled();
  });
});

describe("createQueryClient: после мутации", () => {
  it("успешная мутация перечитывает активные запросы", async () => {
    // Arrange
    const client = createQueryClient(vi.fn());
    const queryFn = vi.fn().mockResolvedValue("данные");
    const observer = new QueryObserver(client, { queryKey: ["status"], queryFn });
    observer.subscribe(() => undefined);
    await vi.waitFor(() => expect(queryFn).toHaveBeenCalledTimes(1));
    const mutation = new MutationObserver(client, { mutationFn: () => Promise.resolve("ok") });

    // Act
    await mutation.mutate(undefined);

    // Assert
    expect(queryFn).toHaveBeenCalledTimes(2);
  });

  it("неудачная мутация запросы не трогает", async () => {
    // Arrange
    const client = createQueryClient(vi.fn());
    const queryFn = vi.fn().mockResolvedValue("данные");
    const observer = new QueryObserver(client, { queryKey: ["status"], queryFn });
    observer.subscribe(() => undefined);
    await vi.waitFor(() => expect(queryFn).toHaveBeenCalledTimes(1));
    const mutation = new MutationObserver(client, {
      mutationFn: rejecting(new ApiProblem({ kind: "problem", status: 409 })),
    });

    // Act
    await mutation.mutate(undefined).catch(() => undefined);

    // Assert
    expect(queryFn).toHaveBeenCalledTimes(1);
  });
});

describe("createQueryClient: повторы", () => {
  it("запрос с 4xx не повторяется (retry по умолчанию из политики)", async () => {
    // Arrange
    const client = createQueryClient(vi.fn());
    const queryFn = vi.fn().mockRejectedValue(new ApiProblem({ kind: "problem", status: 404 }));

    // Act
    await client.fetchQuery({ queryKey: ["x"], queryFn }).catch(() => undefined);

    // Assert
    expect(queryFn).toHaveBeenCalledTimes(1);
  });
});
