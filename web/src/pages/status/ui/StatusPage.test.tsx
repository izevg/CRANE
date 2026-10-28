import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { ApiProblem, getGetStatusQueryOptions } from "~/shared/api";
import { isAutoRefreshPaused, setAutoRefreshPaused } from "~/shared/lib/state";
import { StatusPage } from "./StatusPage";

const UPSTREAM_ROW = /Upstream npmjs/;
const POSTGRES_ROW = /PostgreSQL/;

// Граница — ответы сервера: подменяется queryFn публичных опций операции, остальное настоящее.
const server = vi.hoisted(() => ({ calls: 0, failFrom: 0, mode: undefined as string | undefined }));

vi.mock("~/shared/api", async (original) => {
  const real = await original<typeof import("~/shared/api")>();
  return {
    ...real,
    getGetStatusQueryOptions: () => {
      const options = real.getGetStatusQueryOptions();
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
    },
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
  setAutoRefreshPaused(false);
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
    const updated = screen.getByText("Обновлено");

    // Assert
    expect(updated).toBeInTheDocument();
    expect(updated.querySelector("time")).not.toBeNull();
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

  it("при ошибке обновления компоненты не пропадают", async () => {
    // Arrange
    server.failFrom = 2;
    const client = await renderPage();

    // Act
    await client.refetchQueries({ queryKey: ["status"] });

    // Assert
    expect(
      await screen.findByText("Не удалось обновить данные. Показаны последние полученные"),
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
    expect(isAutoRefreshPaused()).toBe(true);
    expect(screen.getByRole("button", { name: "Пауза автообновления" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });
});
