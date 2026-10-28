import { act, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ApiProblem } from "~/shared/api";
import { renderApp } from "~test/renderApp";

// Граница — ответы сервера для двух операций: подменяется queryFn публичных опций, всё остальное настоящее.
const server = vi.hoisted(() => ({
  meFails: false,
  statusCalls: 0,
  statusFailsFrom: 0,
}));

vi.mock("~/shared/api", async (original) => {
  const real = await original<typeof import("~/shared/api")>();
  const failingStatus = (make: typeof real.getGetStatusQueryOptions) => () => {
    const options = make();
    return {
      ...options,
      queryFn: (context: never) => {
        server.statusCalls += 1;
        if (server.statusFailsFrom > 0 && server.statusCalls >= server.statusFailsFrom) {
          throw new ApiProblem({ kind: "unauthenticated", status: 401 });
        }
        return (options.queryFn as (context: never) => unknown)(context);
      },
    };
  };
  return {
    ...real,
    getGetMeQueryOptions: () => {
      const options = real.getGetMeQueryOptions();
      return {
        ...options,
        queryFn: (context: never) => {
          if (server.meFails) {
            throw new ApiProblem({
              kind: "unauthenticated",
              status: 401,
              reason: "session_expired",
            });
          }
          return (options.queryFn as (context: never) => unknown)(context);
        },
      };
    },
    getGetStatusQueryOptions: failingStatus(real.getGetStatusQueryOptions),
    getGetStatusSuspenseQueryOptions: failingStatus(
      real.getGetStatusSuspenseQueryOptions as typeof real.getGetStatusQueryOptions,
    ),
  };
});

beforeEach(() => {
  server.meFails = false;
  server.statusCalls = 0;
  server.statusFailsFrom = 0;
});

describe("401 при переходе", () => {
  it("профиль не получен: replace на /login с причиной и redirect исходного адреса", async () => {
    // Arrange
    server.meFails = true;
    const { router } = renderApp("/status?window=24h");

    // Act
    await waitFor(() => expect(router.state.location.pathname).toBe("/login"));

    // Assert
    expect(router.state.location.search).toEqual({
      reason: "session_expired",
      redirect: "/status?window=24h",
    });
    expect(
      await screen.findByRole("heading", { level: 1, name: "Войдите, чтобы продолжить" }),
    ).toBeInTheDocument();
  });

  it("preload по наведению с 401 никуда не уводит", async () => {
    // Arrange
    server.meFails = true;
    const { router } = renderApp("/login");
    await screen.findByRole("heading", { level: 1, name: "Войдите, чтобы продолжить" });

    // Act
    await act(() => router.preloadRoute({ to: "/status" }));

    // Assert
    expect(router.state.location.href).toBe("/login");
  });

  it("запись истории заменяется, а не добавляется", async () => {
    // Arrange
    server.meFails = true;
    const { router, history } = renderApp("/status");

    // Act
    await waitFor(() => expect(router.state.location.pathname).toBe("/login"));

    // Assert
    expect(history.length).toBe(1);
  });
});

describe("401 в фоне", () => {
  it("опрос получил 401: баннер «Сессия завершена», страница на месте, действия входа нет", async () => {
    // Arrange
    server.statusFailsFrom = 2;
    const { queryClient, router } = renderApp("/status");
    await screen.findByRole("heading", { level: 1, name: "Статус системы" });

    // Act
    await queryClient.refetchQueries({ queryKey: ["status"] });

    // Assert
    expect(
      await screen.findByText("Сессия завершена. Вход в веб-консоль появится в CRANE 0.4.0."),
    ).toBeInTheDocument();
    expect(screen.queryByRole("link", { name: "Войти снова" })).not.toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/status");
    expect(screen.getByRole("heading", { level: 1, name: "Статус системы" })).toBeInTheDocument();
  });
});
