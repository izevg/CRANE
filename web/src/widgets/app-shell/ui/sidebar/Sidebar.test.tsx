import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { PackageIcon } from "lucide-react";
import { afterEach, describe, expect, it } from "vitest";
import { Sidebar, type SidebarProps } from "./Sidebar";

const THEME_BUTTON = /^Тема:/;

const PROPS: SidebarProps = {
  groups: [
    {
      id: "registry",
      label: "Реестр",
      items: [
        { id: "packages", label: "Пакеты", href: "/packages", icon: PackageIcon, active: true },
        { id: "policies", label: "Политики", href: "/policies" },
      ],
    },
    { id: "bare", items: [{ id: "audit", label: "Журнал", href: "/audit" }] },
  ],
  footerItems: [
    { id: "tokens", label: "Мои токены", href: "/me/tokens" },
    { id: "settings", label: "Настройки", href: "/settings" },
  ],
  helpHref: "/help",
};

afterEach(() => {
  document.documentElement.removeAttribute("data-theme");
  localStorage.clear();
});

describe("Sidebar", () => {
  it("активный пункт помечен aria-current=page, остальные нет", () => {
    // Arrange
    // Act
    render(<Sidebar {...PROPS} collapsed={false} />);

    // Assert
    expect(screen.getByRole("link", { name: "Пакеты" })).toHaveAttribute("aria-current", "page");
    expect(screen.getByRole("link", { name: "Политики" })).not.toHaveAttribute("aria-current");
  });

  it("группы приходят пропсами: подпись группы называет список", () => {
    // Arrange
    // Act
    render(<Sidebar {...PROPS} collapsed={false} />);

    // Assert
    const list = screen.getByRole("list", { name: "Реестр" });
    expect(within(list).getAllByRole("link")).toHaveLength(2);
  });

  it("«Справка» стоит в нижнем блоке и ведёт на helpHref", () => {
    // Arrange
    // Act
    render(<Sidebar {...PROPS} collapsed={false} />);

    // Assert
    const service = screen.getByRole("navigation", { name: "Личное и справка" });
    expect(within(service).getByRole("link", { name: "Справка" })).toHaveAttribute("href", "/help");
  });

  it("«Справка» остаётся в рельсе и после служебных пунктов", () => {
    // Arrange
    // Act
    render(<Sidebar {...PROPS} collapsed />);

    // Assert
    const service = screen.getByRole("navigation", { name: "Личное и справка" });
    const names = within(service)
      .getAllByRole("link")
      .map((link) => link.textContent);
    expect(names).toEqual(["Мои токены", "Настройки", "Справка"]);
  });

  it("в рельсе у пунктов остаются доступные имена", () => {
    // Arrange
    // Act
    render(<Sidebar {...PROPS} collapsed />);

    // Assert
    expect(screen.getByRole("link", { name: "Пакеты" })).toBeInTheDocument();
  });

  it("логотип — единственная ссылка на главную с доступным именем", () => {
    // Arrange
    // Act
    render(<Sidebar {...PROPS} homeHref="/dashboard" collapsed={false} />);

    // Assert
    expect(
      screen.getByRole("link", { name: "CRANE, Реестр артефактов: на главную" }),
    ).toHaveAttribute("href", "/dashboard");
  });

  it("выбор тёмной темы ставит data-theme=dark на документ", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Sidebar {...PROPS} collapsed={false} />);
    await user.click(screen.getByRole("button", { name: THEME_BUTTON }));

    // Act
    await user.click(await screen.findByRole("menuitemradio", { name: "Тёмная" }));

    // Assert
    expect(document.documentElement).toHaveAttribute("data-theme", "dark");
  });

  it("выбранный режим темы — menuitemradio с aria-checked=true", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Sidebar {...PROPS} collapsed={false} />);
    await user.click(screen.getByRole("button", { name: THEME_BUTTON }));

    // Act
    await user.click(await screen.findByRole("menuitemradio", { name: "Тёмная" }));

    // Assert
    expect(screen.getByRole("menuitemradio", { name: "Тёмная" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("menuitemradio", { name: "Светлая" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
  });

  it("имя переключателя темы называет выбранный режим", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Sidebar {...PROPS} collapsed={false} />);
    await user.click(screen.getByRole("button", { name: THEME_BUTTON }));

    // Act
    await user.click(await screen.findByRole("menuitemradio", { name: "Светлая" }));

    // Assert
    expect(screen.getByRole("button", { name: "Тема: Светлая" })).toBeInTheDocument();
  });

  it("без helpHref пункта «Справка» нет: ссылка на несуществующий раздел хуже её отсутствия", () => {
    // Arrange
    const { helpHref: _help, ...withoutHelp } = PROPS;

    // Act
    render(<Sidebar {...withoutHelp} collapsed={false} />);

    // Assert
    const service = screen.getByRole("navigation", { name: "Личное и справка" });
    expect(within(service).queryByRole("link", { name: "Справка" })).not.toBeInTheDocument();
  });

  it("блок пользователя в развёрнутом сайдбаре показывает имя и роль", () => {
    // Arrange
    // Act
    render(
      <Sidebar
        {...PROPS}
        account={{ name: "Иванов И.", role: "владелец библиотеки" }}
        collapsed={false}
      />,
    );

    // Assert
    expect(screen.getByText("Иванов И.")).toBeInTheDocument();
    expect(screen.getByText("владелец библиотеки")).toBeInTheDocument();
  });

  it("в рельсе остаётся только аватар с доступным именем из имени и роли", () => {
    // Arrange
    // Act
    render(
      <Sidebar {...PROPS} account={{ name: "Иванов И.", role: "владелец библиотеки" }} collapsed />,
    );

    // Assert
    expect(screen.getByRole("img", { name: "Иванов И., владелец библиотеки" })).toBeInTheDocument();
    expect(screen.queryByText("владелец библиотеки")).not.toBeInTheDocument();
  });
});
