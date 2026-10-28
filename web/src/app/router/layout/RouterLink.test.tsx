import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Link, LinkProvider } from "~/shared/ui/primitives";
import { RouterLink } from "./RouterLink";

function renderLinks() {
  const root = createRootRoute({
    component: () => (
      <LinkProvider component={RouterLink}>
        <Link href="/status">{"Статус"}</Link>
        <Link href="/status?window=24h">{"Статус за сутки"}</Link>
        <Link href="/packages/npm/left-pad">{"Пакет"}</Link>
        <Link href="/help">{"Справка"}</Link>
        <Link href="https://docs.example/crane">{"Документация"}</Link>
        <Link href="//cdn.example/x">{"Без схемы"}</Link>
        <Outlet />
      </LinkProvider>
    ),
  });
  const home = createRoute({
    getParentRoute: () => root,
    path: "/",
    component: () => <p>{"Главная"}</p>,
  });
  const status = createRoute({
    getParentRoute: () => root,
    path: "/status",
    validateSearch: ({ window: range }: Record<string, unknown>) =>
      typeof range === "string" ? { window: range } : {},
    component: () => <p>{"Страница статуса"}</p>,
  });
  const pkg = createRoute({
    getParentRoute: () => root,
    path: "/packages/$format/$name",
    component: () => <p>{"Карточка пакета"}</p>,
  });
  const router = createRouter({
    routeTree: root.addChildren([home, status, pkg]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  render(<RouterProvider router={router} />);
  return router;
}

describe("RouterLink: адреса внутри приложения", () => {
  it("обычный клик по адресу с маршрутом — клиентский переход без перезагрузки документа", async () => {
    // Arrange
    const user = userEvent.setup();
    const router = renderLinks();
    const link = await screen.findByRole("link", { name: "Статус" });

    // Act
    await user.click(link);

    // Assert
    await waitFor(() => expect(router.state.location.pathname).toBe("/status"));
    expect(await screen.findByText("Страница статуса")).toBeInTheDocument();
  });

  it("клик по ссылке с query переходит внутри SPA и сохраняет query", async () => {
    // Arrange
    const router = renderLinks();
    const link = await screen.findByRole("link", { name: "Статус за сутки" });

    // Act
    const notPrevented = fireEvent.click(link);

    // Assert
    expect(notPrevented).toBe(false);
    await waitFor(() => expect(router.state.location.href).toBe("/status?window=24h"));
  });

  it("клик по ссылке на маршрут с параметрами пути переходит внутри SPA", async () => {
    // Arrange
    const router = renderLinks();
    const link = await screen.findByRole("link", { name: "Пакет" });

    // Act
    const notPrevented = fireEvent.click(link);

    // Assert
    expect(notPrevented).toBe(false);
    await waitFor(() => expect(router.state.location.pathname).toBe("/packages/npm/left-pad"));
    expect(await screen.findByText("Карточка пакета")).toBeInTheDocument();
  });

  it("на странице маршрута ссылка получает aria-current=page", async () => {
    // Arrange
    const user = userEvent.setup();
    renderLinks();
    await user.click(await screen.findByRole("link", { name: "Статус" }));

    // Act
    const link = await screen.findByRole("link", { name: "Статус", current: "page" });

    // Assert
    expect(link).toHaveAttribute("href", "/status");
  });

  it("клик с Ctrl оставляет поведение браузера (новая вкладка)", async () => {
    // Arrange
    const router = renderLinks();
    const link = await screen.findByRole("link", { name: "Статус" });

    // Act
    const notPrevented = fireEvent.click(link, { ctrlKey: true });

    // Assert
    expect(notPrevented).toBe(true);
    expect(router.state.location.pathname).toBe("/");
  });
});

describe("RouterLink: чужие адреса", () => {
  it.each([
    ["со схемой", "Документация", "https://docs.example/crane"],
    ["без схемы, с //", "Без схемы", "//cdn.example/x"],
  ])("%s — обычный <a>, роутер не перехватывает клик", async (_name, label, href) => {
    // Arrange
    const router = renderLinks();
    const link = await screen.findByRole("link", { name: label });

    // Act
    const notPrevented = fireEvent.click(link);

    // Assert
    expect(link).toHaveAttribute("href", href);
    expect(notPrevented).toBe(true);
    expect(router.state.location.pathname).toBe("/");
  });
});
