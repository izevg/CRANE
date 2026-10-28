import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiProblem, getGetStatusQueryOptions } from "~/shared/api";
import { announce } from "~/shared/lib/a11y";
import { autoRefreshPaused } from "~/shared/lib/state";
import { StatusPage } from "./StatusPage";

const UPDATED = /^Обновлено /;
const UPSTREAM_ROW = /Upstream npmjs/;
const POSTGRES_ROW = /PostgreSQL/;

// Граница — ответы сервера: подменяется queryFn публичных опций операции, остальное настоящее.
const server = vi.hoisted(() => ({
  calls: 0,
  failFrom: 0,
  mode: undefined as string | undefined,
}));

vi.mock("~/shared/lib/a11y", () => ({ announce: vi.fn() }));

vi.mock("~/shared/api", async (original) => {
  const real = await original<typeof import("~/shared/api")>();
  const patched = (make: typeof real.getGetStatusQueryOptions) => () => {
    const options = make();
    return {
      ...options,
      queryFn: async (context: never) => {
        server.calls += 1;
        if (server.failFrom > 0 && server.calls >= server.failFrom) {
          throw new ApiProblem({ kind: "problem", status: 500, title: "Сбой" });
        }
        const response = (await (options.queryFn as (context: never) => unknown)(context)) as {
          data: object;
        };
        return server.mode === undefined
          ? response
          : { ...response, data: { ...response.data, mode: server.mode } };
      },
    };
  };
  return {
    ...real,
    getGetStatusQueryOptions: patched(real.getGetStatusQueryOptions),
    getGetStatusSuspenseQueryOptions: patched(
      real.getGetStatusSuspenseQueryOptions as typeof real.getGetStatusQueryOptions,
    ),
  };
});

async function renderPage() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false, staleTime: Number.POSITIVE_INFINITY } },
  });
  // как loader маршрута: данные уже в кэше к моменту показа страницы
  await client.prefetchQuery(getGetStatusQueryOptions());
  render(
    <QueryClientProvider client={client}>
      <StatusPage />
    </QueryClientProvider>,
  );
  return client;
}

const MODES = [
  ["normal", "Режим: норма", "success"],
  ["read_only", "Режим: только чтение", "warning"],
  ["degraded", "Режим: деградация", "warning"],
] as const;

afterEach(() => {
  server.calls = 0;
  server.failFrom = 0;
  server.mode = undefined;
  vi.mocked(announce).mockClear();
  autoRefreshPaused.set(false);
});

describe("StatusPage: режим платформы", () => {
  it.each(MODES)("%s — «%s», тон %s", async (mode, label, tone) => {
    // Arrange
    server.mode = mode;
    await renderPage();

    // Act
    const badge = screen.getByText(label).closest("[data-tone]");

    // Assert
    expect(badge).toHaveAttribute("data-tone", tone);
  });
});

describe("StatusPage: данные", () => {
  it("режим платформы в шапке — бейдж с текстом", async () => {
    // Arrange
    await renderPage();

    // Act
    const heading = screen.getByRole("heading", { level: 1, name: "Статус системы" });

    // Assert
    expect(heading).toBeInTheDocument();
    expect(screen.getByText("Режим: деградация")).toBeInTheDocument();
  });

  it("плитки: версия и отставание в минутах, секундах", async () => {
    // Arrange
    await renderPage();

    // Act
    const feeds = screen.getByText("Отставание фидов").closest("[data-slot]");

    // Assert
    expect(screen.getByText("0.3.0")).toBeInTheDocument();
    expect(feeds).toHaveTextContent("9");
    expect(feeds).toHaveTextContent("мин");
  });

  it("компоненты: состояние текстом, задержка с единицей, без данных «—»", async () => {
    // Arrange
    await renderPage();

    // Act
    const row = screen.getByRole("row", { name: UPSTREAM_ROW });

    // Assert
    expect(within(row).getByText("Недоступен")).toBeInTheDocument();
    expect(within(row).getByText("—")).toBeInTheDocument();
    expect(
      within(screen.getByRole("row", { name: POSTGRES_ROW })).getByText("4 мс"),
    ).toBeInTheDocument();
  });

  it("деградации перечислены понятным текстом", async () => {
    // Arrange
    await renderPage();

    // Act
    const list = screen
      .getByText("Часть возможностей работает с ограничениями")
      .closest("[data-slot=callout]");

    // Assert
    expect(list).toHaveTextContent("Семантический поиск недоступен");
    expect(list).toHaveTextContent("Upstream npmjs недоступен");
  });
});

describe("StatusPage: обновление", () => {
  it("«Обновлено» показывает возраст данных", async () => {
    // Arrange
    await renderPage();

    // Act
    const updated = screen.getByText(UPDATED);

    // Assert
    expect(updated.tagName).toBe("TIME");
    expect(updated).toHaveAttribute("datetime");
  });

  it("ошибка очередного обновления: данные остаются, есть «Повторить», повтор перечитывает", async () => {
    // Arrange
    server.failFrom = 2;
    const user = userEvent.setup();
    const client = await renderPage();
    await client.refetchQueries({ queryKey: ["status"] });
    const retry = await screen.findByRole("button", { name: "Повторить" });
    server.failFrom = 0;

    // Act
    await user.click(retry);

    // Assert
    await waitFor(() =>
      expect(screen.queryByRole("button", { name: "Повторить" })).not.toBeInTheDocument(),
    );
    expect(screen.getByText("Scanner worker")).toBeInTheDocument();
  });

  it("повтор не удался — объявляется один раз", async () => {
    // Arrange
    server.failFrom = 2;
    const user = userEvent.setup();
    const client = await renderPage();
    await client.refetchQueries({ queryKey: ["status"] });
    const retry = await screen.findByRole("button", { name: "Повторить" });

    // Act
    await user.click(retry);

    // Assert
    await waitFor(() => expect(announce).toHaveBeenCalledTimes(1));
    expect(announce).toHaveBeenCalledWith("Не удалось обновить данные. Попробуйте ещё раз.");
  });

  it("пока «Повторить» не нажата, сбой опроса ничего не объявляет", async () => {
    // Arrange
    server.failFrom = 2;
    const client = await renderPage();

    // Act
    await client.refetchQueries({ queryKey: ["status"] });

    // Assert
    expect(await screen.findByRole("button", { name: "Повторить" })).toBeInTheDocument();
    expect(announce).not.toHaveBeenCalled();
  });

  it("при ошибке обновления компоненты не пропадают", async () => {
    // Arrange
    server.failFrom = 2;
    const client = await renderPage();

    // Act
    await client.refetchQueries({ queryKey: ["status"] });

    // Assert
    expect(
      await screen.findByText("Не удалось обновить данные. Показаны последние полученные."),
    ).toBeInTheDocument();
    expect(screen.getByText("Scanner worker")).toBeInTheDocument();
  });

  it("«Пауза автообновления» переключает общий флаг", async () => {
    // Arrange
    const user = userEvent.setup();
    await renderPage();

    // Act
    await user.click(screen.getByRole("button", { name: "Пауза автообновления" }));

    // Assert
    expect(autoRefreshPaused.get()).toBe(true);
    expect(screen.getByRole("button", { name: "Пауза автообновления" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });
});
