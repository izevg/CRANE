import {
  createMemoryHistory,
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  RouterProvider,
} from "@tanstack/react-router";
import { render, screen, waitFor } from "@testing-library/react";
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
        <Link href="/help">{"Справка"}</Link>
        <Link href="https://docs.example/crane">{"Документация"}</Link>
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
    component: () => <p>{"Страница статуса"}</p>,
  });
  const router = createRouter({
    routeTree: root.addChildren([home, status]),
    history: createMemoryHistory({ initialEntries: ["/"] }),
  });
  render(<RouterProvider router={router} />);
  return router;
}

describe("RouterLink", () => {
  it("обычный клик по адресу с маршрутом — клиентский переход без перезагрузки", async () => {
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
    const user = userEvent.setup();
    const router = renderLinks();
    const link = await screen.findByRole("link", { name: "Статус" });
    await user.keyboard("{Control>}");

    // Act
    await user.click(link);

    // Assert
    expect(router.state.location.pathname).toBe("/");
  });

  it.each([
    ["адрес с query", "Статус за сутки", "/status?window=24h"],
    ["адрес без маршрута", "Справка", "/help"],
    ["чужой адрес", "Документация", "https://docs.example/crane"],
  ])("%s — обычный <a href>", async (_name, label, href) => {
    // Arrange
    renderLinks();

    // Act
    const link = await screen.findByRole("link", { name: label });

    // Assert
    expect(link).toHaveAttribute("href", href);
  });
});
