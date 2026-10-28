import { act, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp } from "~test/renderApp";

const PAUSE = "Пауза автообновления";

describe("маршруты: переходы", () => {
  it("«/» заменяет запись истории на /status", async () => {
    // Arrange
    const { router, history } = renderApp("/");

    // Act
    await screen.findByRole("heading", { level: 1, name: "Статус системы" });

    // Assert
    expect(router.state.location.pathname).toBe("/status");
    expect(history.length).toBe(1);
  });

  it("переход на «/» из другого раздела не оставляет «/» в истории", async () => {
    // Arrange
    const { router, history } = renderApp("/login");
    await screen.findByRole("heading", { level: 1, name: "Войдите, чтобы продолжить" });

    // Act
    await act(() => router.navigate({ to: "/" }));

    // Assert
    await screen.findByRole("heading", { level: 1, name: "Статус системы" });
    expect(history.length).toBe(2);
  });

  it("E15: данные заглушки, плашка демо, заголовок документа", async () => {
    // Arrange
    renderApp("/status");

    // Act
    const heading = await screen.findByRole("heading", { level: 1, name: "Статус системы" });

    // Assert
    expect(heading).toBeInTheDocument();
    expect(await screen.findByText("Scanner worker")).toBeInTheDocument();
    expect(document.title).toBe("Статус системы — CRANE");
  });

  it("плашка «Демо-данные» одна и стоит внутри main над заголовком страницы", async () => {
    // Arrange
    renderApp("/status");
    await screen.findByRole("heading", { level: 1, name: "Статус системы" });

    // Act
    const notes = screen.getAllByRole("note");

    // Assert
    expect(notes).toHaveLength(1);
    expect(within(screen.getByRole("main")).getByRole("note")).toHaveTextContent("Демо-данные");
  });

  it("сайдбар строится из _links GET /me и помечает текущий раздел aria-current", async () => {
    // Arrange
    renderApp("/status");
    await screen.findByRole("heading", { level: 1, name: "Статус системы" });

    // Act
    const nav = screen.getByRole("navigation", { name: "Основное меню" });

    // Assert
    const current = within(nav).getByRole("link", { name: "Статус системы" });
    expect(current).toHaveAttribute("aria-current", "page");
    // в ответе есть ещё crane:search и crane:my-tokens, но маршрутов под них нет: пунктов нет
    expect(within(nav).getAllByRole("link")).toHaveLength(1);
  });

  it("неизвестный адрес: 404 внутри шелла, URL не меняется, заголовок документа нейтральный", async () => {
    // Arrange
    const { router } = renderApp("/nope/at/all");

    // Act
    const heading = await screen.findByRole("heading", {
      level: 1,
      name: "Не найдено или нет доступа",
    });

    // Assert
    expect(within(screen.getByRole("main")).getByText("Не найдено или нет доступа")).toBe(heading);
    expect(router.state.location.pathname).toBe("/nope/at/all");
    expect(screen.getByRole("navigation", { name: "Основное меню" })).toBeInTheDocument();
    expect(document.title).toBe("Не найдено или нет доступа — CRANE");
  });

  it("/login: без шелла, h1 «Войдите, чтобы продолжить», кнопки входа нет, плашка демо одна", async () => {
    // Arrange
    renderApp("/login");

    // Act
    const heading = await screen.findByRole("heading", {
      level: 1,
      name: "Войдите, чтобы продолжить",
    });

    // Assert
    expect(heading).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "Основное меню" })).not.toBeInTheDocument();
    expect(screen.getByText("Вход появится в следующей версии CRANE")).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
    expect(screen.getAllByRole("note")).toHaveLength(1);
    expect(screen.getAllByRole("main")).toHaveLength(1);
  });

  it("/login с причиной показывает её подсказку, а redirect на чужой адрес сводится к «/»", async () => {
    // Arrange
    const { router } = renderApp("/login?reason=session_revoked&redirect=//evil.example");

    // Act
    const heading = await screen.findByRole("heading", { level: 1, name: "Сессия завершена" });

    // Assert
    expect(heading).toBeInTheDocument();
    expect(router.state.location.search).toEqual({ reason: "session_revoked", redirect: "/" });
  });
});

describe("маршруты: фокус", () => {
  it("смена pathname переносит фокус на h1 новой страницы", async () => {
    // Arrange
    const { router } = renderApp("/status");
    await screen.findByRole("heading", { level: 1, name: "Статус системы" });
    screen.getByRole("button", { name: PAUSE }).focus();

    // Act
    await act(() => router.navigate({ href: "/there-is-nothing" }));

    // Assert
    const heading = await screen.findByRole("heading", {
      level: 1,
      name: "Не найдено или нет доступа",
    });
    await waitFor(() => expect(heading).toHaveFocus());
  });

  it("смена только search фокус не трогает", async () => {
    // Arrange
    const { router } = renderApp("/status");
    await screen.findByRole("heading", { level: 1, name: "Статус системы" });
    const toggle = screen.getByRole("button", { name: PAUSE });
    toggle.focus();

    // Act
    await act(() => router.navigate({ href: "/status?window=24h" }));

    // Assert
    await waitFor(() => expect(router.state.location.search).toEqual({ window: "24h" }));
    expect(toggle).toHaveFocus();
  });

  it("первая загрузка документа не переносит фокус", async () => {
    // Arrange
    renderApp("/status");

    // Act
    await screen.findByRole("heading", { level: 1, name: "Статус системы" });

    // Assert
    expect(document.body).toHaveFocus();
  });
});
