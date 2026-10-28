import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { loginRoute } from "~/pages/login";
import { ApiProblem } from "~/shared/api";
import { RouteError } from "./RouteError";

// Расписание паузы проверено в backoff.test.ts; здесь пауза короткая, чтобы не ждать и не подменять таймеры
// (testing-library опирается на настоящие).
const pause = vi.hoisted(() => ({ ms: 0 }));
vi.mock("../recovery/backoff", () => ({
  appBackoff: { next: () => pause.ms, reset: () => undefined },
}));

const SHORT_MS = 30;
const TOO_EARLY_MS = 600_000;
const OBSERVE_MS = 150;
const STATUS = { unavailable: 503, unauthorized: 401 } as const;

/** Loader, который падает первые `failures` раз, а потом отдаёт страницу. */
function renderFlaky(failures: number, problem: () => ApiProblem) {
  const calls = { loader: 0 };
  const root = createRootRoute({
    component: () => (
      <>
        <button type="button">{"Вне main"}</button>
        <main>
          <Outlet />
        </main>
      </>
    ),
  });
  const page = createRoute({
    getParentRoute: () => root,
    path: "/page",
    loader: () => {
      calls.loader += 1;
      if (calls.loader <= failures) {
        throw problem();
      }
    },
    component: () => <h1>{"Страница открылась"}</h1>,
  });
  const login = createRoute({
    getParentRoute: () => root,
    path: "/login",
    validateSearch: loginRoute.validateSearch,
    component: () => <h1>{"Вход"}</h1>,
  });
  const router = createRouter({
    routeTree: root.addChildren([page, login]),
    history: createMemoryHistory({ initialEntries: ["/page"] }),
    defaultErrorComponent: RouteError,
    defaultPendingMinMs: 0,
  });
  render(<RouterProvider router={router} />);
  return { calls, router };
}

const unavailable = () => new ApiProblem({ kind: "problem", status: STATUS.unavailable });
const offline = () => new ApiProblem({ kind: "network", status: 0 });

beforeEach(() => {
  pause.ms = SHORT_MS;
});

describe("RouteError: автоповтор при недоступном сервере", () => {
  it("через 5 с маршрут перезапускается, и страница открывается", async () => {
    // Arrange
    renderFlaky(1, unavailable);
    await screen.findByRole("heading", { level: 1, name: "Сервер CRANE не отвечает" });

    // Act
    const opened = await screen.findByRole("heading", { name: "Страница открылась" });

    // Assert
    expect(opened).toBeInTheDocument();
  });

  it("раньше срока маршрут не перезапускается", async () => {
    // Arrange
    pause.ms = TOO_EARLY_MS;
    const { calls } = renderFlaky(1, unavailable);
    await screen.findByRole("heading", { level: 1, name: "Сервер CRANE не отвечает" });

    // Act
    await act(() => new Promise((resolve) => setTimeout(resolve, OBSERVE_MS)));

    // Assert
    expect(calls.loader).toBe(1);
  });

  it("событие online перезапускает маршрут сразу", async () => {
    // Arrange
    pause.ms = TOO_EARLY_MS;
    renderFlaky(1, offline);
    await screen.findByRole("heading", { level: 1, name: "Сервер CRANE не отвечает" });

    // Act
    await act(async () => {
      window.dispatchEvent(new Event("online"));
    });

    // Assert
    expect(await screen.findByRole("heading", { name: "Страница открылась" })).toBeInTheDocument();
  });

  it("ошибка сервера без недоступности (500) сама не повторяется", async () => {
    // Arrange
    const { calls } = renderFlaky(1, () => new ApiProblem({ kind: "problem", status: 500 }));
    await screen.findByRole("heading", { level: 1 });

    // Act
    await act(() => new Promise((resolve) => setTimeout(resolve, OBSERVE_MS)));

    // Assert
    expect(calls.loader).toBe(1);
  });
});

describe("RouteError: 401 при переходе", () => {
  it("replace на /login с причиной и адресом, с которого ушли", async () => {
    // Arrange
    const { router } = renderFlaky(
      1,
      () =>
        new ApiProblem({
          kind: "unauthenticated",
          status: STATUS.unauthorized,
          reason: "session_expired",
        }),
    );

    // Act
    await screen.findByRole("heading", { name: "Вход" });

    // Assert
    expect(router.state.location.pathname).toBe("/login");
    expect(router.state.location.search).toEqual({ reason: "session_expired", redirect: "/page" });
  });
});

describe("RouteError: фокус при повторе", () => {
  it("ручной повтор: фокус, стоявший в блоке ошибки, оказывается на h1 результата", async () => {
    // Arrange
    pause.ms = TOO_EARLY_MS;
    const user = userEvent.setup();
    renderFlaky(2, unavailable);
    const retry = await screen.findByRole("button", { name: "Повторить" });
    retry.focus();

    // Act
    await user.keyboard("{Enter}");

    // Assert
    const heading = await screen.findByRole("heading", {
      level: 1,
      name: "Сервер CRANE не отвечает",
    });
    await waitFor(() => expect(heading).toHaveFocus());
  });

  it("два автоповтора с фокусом на «Повторить»: каждый раз фокус на h1, а не на body", async () => {
    // Arrange
    renderFlaky(2, unavailable);
    const retry = await screen.findByRole("button", { name: "Повторить" });
    retry.focus();

    // Act
    await screen.findByRole("heading", { level: 1, name: "Страница открылась" });

    // Assert
    await waitFor(() =>
      expect(screen.getByRole("heading", { level: 1, name: "Страница открылась" })).toHaveFocus(),
    );
    expect(document.body).not.toHaveFocus();
  });

  it("фокус вне main (сайдбар, topbar) повтор не трогает", async () => {
    // Arrange
    pause.ms = SHORT_MS;
    renderFlaky(1, unavailable);
    await screen.findByRole("heading", { level: 1, name: "Сервер CRANE не отвечает" });
    const outside = screen.getByRole("button", { name: "Вне main" });
    outside.focus();
    await screen.findByRole("heading", { level: 1, name: "Страница открылась" });

    // Act
    await act(() => new Promise((resolve) => setTimeout(resolve, OBSERVE_MS)));

    // Assert
    expect(outside).toHaveFocus();
  });
});
