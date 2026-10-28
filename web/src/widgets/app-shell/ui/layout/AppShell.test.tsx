import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { AppShell, type Banner, type SidebarProps, type TopbarProps } from "~/widgets/app-shell";
import { stubMatchMedia } from "~test/matchMedia";

const WIDE = ["(min-width: 768px)", "(min-width: 1024px)"];
const MEDIUM = ["(min-width: 768px)"];

const sidebar: SidebarProps = {
  groups: [
    { id: "registry", label: "Реестр", items: [{ id: "packages", label: "Каталог", href: "/c" }] },
  ],
  footerItems: [],
  helpHref: "/help",
};
const topbar: TopbarProps = {
  breadcrumbs: { trail: [{ label: "Пакеты", href: "/p" }], current: "left-pad" },
  onOpenPalette: vi.fn(),
};

function renderShell(banners: readonly Banner[] = []) {
  return render(
    <AppShell sidebar={sidebar} topbar={topbar} banners={banners}>
      <h1>{"Страница"}</h1>
    </AppShell>,
  );
}

describe("AppShell", () => {
  it("skip-link «К содержимому» — первый элемент в порядке Tab", async () => {
    // Arrange
    stubMatchMedia(WIDE);
    const user = userEvent.setup();
    renderShell();

    // Act
    await user.tab();

    // Assert
    expect(screen.getByRole("link", { name: "К содержимому" })).toHaveFocus();
  });

  it("Enter на skip-link переводит фокус на main", async () => {
    // Arrange
    stubMatchMedia(WIDE);
    const user = userEvent.setup();
    renderShell();
    await user.tab();

    // Act
    await user.keyboard("{Enter}");

    // Assert
    expect(screen.getByRole("main")).toHaveFocus();
  });

  it("содержимое страницы лежит внутри main", () => {
    // Arrange
    stubMatchMedia(WIDE);

    // Act
    renderShell();

    // Assert
    expect(screen.getByRole("main")).toContainElement(
      screen.getByRole("heading", { name: "Страница" }),
    );
  });

  it("на широком экране кнопка меню сворачивает сайдбар и меняет aria-expanded", async () => {
    // Arrange
    stubMatchMedia(WIDE);
    const user = userEvent.setup();
    renderShell();
    const toggle = screen.getByRole("button", { name: "Главное меню" });

    // Act
    await user.click(toggle);

    // Assert
    expect(toggle).toHaveAttribute("aria-expanded", "false");
  });

  it("на широком экране сайдбар развёрнут: aria-expanded=true и видна подпись группы", () => {
    // Arrange
    stubMatchMedia(WIDE);

    // Act
    renderShell();

    // Assert
    expect(screen.getByRole("button", { name: "Главное меню" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
    expect(screen.getByText("Реестр")).toBeVisible();
  });

  it("на средней ширине сайдбар свёрнут принудительно и кнопки меню нет", () => {
    // Arrange
    stubMatchMedia(MEDIUM);

    // Act
    renderShell();

    // Assert
    expect(screen.queryByRole("button", { name: "Главное меню" })).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Каталог" })).toBeInTheDocument();
  });

  it("на узком экране меню закрыто, а кнопка открывает его в диалоге", async () => {
    // Arrange
    stubMatchMedia();
    const user = userEvent.setup();
    renderShell();

    // Act
    await user.click(screen.getByRole("button", { name: "Главное меню" }));

    // Assert
    const dialog = await screen.findByRole("dialog", { name: "Главное меню" });
    expect(dialog).toHaveTextContent("Каталог");
  });

  it("баннеры выводятся под topbar, не больше двух", () => {
    // Arrange
    stubMatchMedia(WIDE);
    const banners: Banner[] = [
      { id: "1", tone: "error", text: "Первый" },
      { id: "2", tone: "warning", text: "Второй" },
      { id: "3", tone: "info", text: "Третий" },
    ];

    // Act
    renderShell(banners);

    // Assert
    expect(screen.getByText("Второй")).toBeInTheDocument();
    expect(screen.queryByText("Третий")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ещё 1" })).toBeInTheDocument();
  });
});
