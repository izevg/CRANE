import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  notFound,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ApiProblem } from "~/shared/api";
import { lazyPage } from "../chunk/chunk";
import { reloadPage } from "../chunk/reload";
import { NotFoundPage } from "./notFound";
import { RouteError } from "./RouteError";

vi.mock("../chunk/reload", () => ({ reloadPage: vi.fn() }));

const CHUNK_MESSAGE =
  "Failed to fetch dynamically imported module: https://crane.example/assets/Page.js";
const REQUEST_ID = /req-42/;
const SECRET = /секретная/;
const RULE_LINE = /Правило/;
const RELOAD_KEY = "crane.chunk-reload-at";
const STATUS = { notFound: 404, serverError: 500, unavailable: 503 } as const;

type Spec = {
  component?: () => never;
  loader?: () => never | Promise<never>;
  lazy?: () => Promise<never>;
};

function renderRoute(spec: Spec) {
  const root = createRootRoute({ component: Outlet, notFoundComponent: NotFoundPage });
  const index = createRoute({
    getParentRoute: () => root,
    path: "/",
    component: () => <p>{"Главная"}</p>,
  });
  const broken = createRoute({
    getParentRoute: () => root,
    path: "/broken",
    ...(spec.loader ? { loader: spec.loader } : {}),
    component: spec.lazy
      ? lazyPage(spec.lazy as never)
      : (spec.component ?? (() => <p>{"Страница"}</p>)),
  });
  const router = createRouter({
    routeTree: root.addChildren([index, broken]),
    history: createMemoryHistory({ initialEntries: ["/broken"] }),
    defaultErrorComponent: RouteError,
    defaultPendingMinMs: 0,
  });
  return render(<RouterProvider router={router} />);
}

function failedChunk(): Promise<never> {
  return Promise.reject(new TypeError(CHUNK_MESSAGE));
}

beforeEach(() => {
  sessionStorage.clear();
  vi.mocked(reloadPage).mockClear();
  vi.spyOn(navigator, "onLine", "get").mockReturnValue(true);
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("RouteError: сбой чанка маршрута", () => {
  it("первый сбой — одна автоматическая перезагрузка и метка в sessionStorage", async () => {
    // Arrange
    renderRoute({ lazy: failedChunk });

    // Act
    await vi.waitFor(() => expect(reloadPage).toHaveBeenCalled());

    // Assert
    expect(reloadPage).toHaveBeenCalledTimes(1);
    expect(sessionStorage.getItem(RELOAD_KEY)).not.toBeNull();
    expect(screen.queryByRole("heading")).not.toBeInTheDocument();
  });

  it("повторный сбой в окне — экран «Не удалось загрузить раздел» и никакой новой перезагрузки", async () => {
    // Arrange
    sessionStorage.setItem(RELOAD_KEY, String(Date.now()));
    renderRoute({ lazy: failedChunk });

    // Act
    const heading = await screen.findByRole("heading", {
      level: 1,
      name: "Не удалось загрузить раздел",
    });

    // Assert
    expect(heading).toBeInTheDocument();
    expect(screen.getByText("Возможно, CRANE обновился. Обновите страницу.")).toBeInTheDocument();
    expect(reloadPage).not.toHaveBeenCalled();
  });

  it("«Обновить страницу» перезагружает по явному действию пользователя", async () => {
    // Arrange
    sessionStorage.setItem(RELOAD_KEY, String(Date.now()));
    const user = userEvent.setup();
    renderRoute({ lazy: failedChunk });
    const button = await screen.findByRole("button", { name: "Обновить страницу" });

    // Act
    await user.click(button);

    // Assert
    expect(reloadPage).toHaveBeenCalledTimes(1);
  });

  it("ошибка при вычислении модуля страницы — не сбой чанка: без перезагрузки", async () => {
    // Arrange
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    renderRoute({ lazy: () => Promise.reject(new Error("синтаксис модуля")) });

    // Act
    const heading = await screen.findByRole("heading", {
      level: 1,
      name: "Не удалось открыть страницу",
    });

    // Assert
    expect(heading).toBeInTheDocument();
    expect(reloadPage).not.toHaveBeenCalled();
  });

  it("нет сети — «Нет соединения» с «Повторить», перезагрузки нет", async () => {
    // Arrange
    vi.spyOn(navigator, "onLine", "get").mockReturnValue(false);
    renderRoute({ lazy: failedChunk });

    // Act
    const heading = await screen.findByRole("heading", { level: 1, name: "Нет соединения" });

    // Assert
    expect(heading).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Повторить" })).toBeInTheDocument();
    expect(reloadPage).not.toHaveBeenCalled();
  });
});

describe("RouteError: ошибки loader'а и рендера", () => {
  it("problem+json 500 — заголовок сервера, идентификатор запроса и «Повторить»", async () => {
    // Arrange
    const problem = new ApiProblem({
      kind: "problem",
      status: STATUS.serverError,
      title: "Хранилище недоступно",
      requestId: "req-42",
    });
    renderRoute({
      loader: () => {
        throw problem;
      },
    });

    // Act
    const heading = await screen.findByRole("heading", { level: 1, name: "Хранилище недоступно" });

    // Assert
    expect(heading).toBeInTheDocument();
    expect(screen.getByRole("button", { name: REQUEST_ID })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Повторить" })).toBeInTheDocument();
  });

  it("503 — «Сервер CRANE недоступен»", async () => {
    // Arrange
    renderRoute({
      loader: () => {
        throw new ApiProblem({ kind: "problem", status: STATUS.unavailable });
      },
    });

    // Act
    const heading = await screen.findByRole("heading", {
      level: 1,
      name: "Сервер CRANE недоступен",
    });

    // Assert
    expect(heading).toBeInTheDocument();
  });

  it("исключение при рендере — «Не удалось открыть страницу» без текста исключения", async () => {
    // Arrange
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    renderRoute({
      component: () => {
        throw new Error("секретная внутренность стека");
      },
    });

    // Act
    const heading = await screen.findByRole("heading", {
      level: 1,
      name: "Не удалось открыть страницу",
    });

    // Assert
    expect(heading).toBeInTheDocument();
    expect(screen.queryByText(SECRET)).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Обновить страницу" })).toBeInTheDocument();
  });

  it("сбой чанка внутри страницы (ошибка рендера) страницу не перезагружает", async () => {
    // Arrange
    vi.spyOn(console, "error").mockImplementation(() => undefined);
    renderRoute({
      component: () => {
        throw new TypeError(CHUNK_MESSAGE);
      },
    });

    // Act
    const heading = await screen.findByRole("heading", {
      level: 1,
      name: "Не удалось открыть страницу",
    });

    // Assert
    expect(heading).toBeInTheDocument();
    expect(reloadPage).not.toHaveBeenCalled();
  });
});

describe("404 из двух источников", () => {
  function section(container: HTMLElement): string {
    const html = container.querySelector("section")?.outerHTML ?? "";
    // useId даёт разные id при разных рендерах: на сходство разметки они не влияют
    return html.replace(/ (id|aria-labelledby)="[^"]*"/g, "");
  }

  it("ответ API 404 и адрес без маршрута дают одинаковую разметку", async () => {
    // Arrange
    const viaLoader = renderRoute({
      loader: () => {
        throw new ApiProblem({ kind: "problem", status: STATUS.notFound, requestId: "req-1" });
      },
    });
    await within(viaLoader.container).findByRole("heading", { name: "Не найдено или нет доступа" });
    const fromLoader = section(viaLoader.container);
    viaLoader.unmount();
    const root = createRootRoute({ component: Outlet, notFoundComponent: NotFoundPage });
    const router = createRouter({
      routeTree: root.addChildren([]),
      history: createMemoryHistory({ initialEntries: ["/нет-такого"] }),
    });
    const unmatched = render(<RouterProvider router={router} />);
    await within(unmatched.container).findByRole("heading", { name: "Не найдено или нет доступа" });

    // Act
    const fromUnmatched = section(unmatched.container);

    // Assert
    expect(fromUnmatched).toBe(fromLoader);
  });

  it("notFound() из loader попадает на тот же экран", async () => {
    // Arrange
    renderRoute({
      loader: () => {
        throw notFound();
      },
    });

    // Act
    const heading = await screen.findByRole("heading", {
      level: 1,
      name: "Не найдено или нет доступа",
    });

    // Assert
    expect(heading).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "На главную" })).toBeInTheDocument();
    expect(screen.queryByText(RULE_LINE)).not.toBeInTheDocument();
  });
});
