import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Action } from "~/shared/ui/actions";
import { CommandPaletteView } from "~/widgets/command-palette";

const UNBLOCK = /Снять блок/;
const NAV_PACKAGES = /Пакеты и репозитории/;
const LEFT_PAD = /left-pad/;
const STEP_UP = /Отозвать токен.*требуется подтверждение входа/;
const NAME = "Перейти, найти пакет или действие";

const navigation = [
  { id: "packages", label: "Пакеты и репозитории", href: "/packages", hint: "Раздел" },
  { id: "audit", label: "Журнал аудита", href: "/audit" },
];

const packages = [
  { id: "p1", name: "left-pad", format: "npm", latest: "1.3.0", href: "/packages/left-pad" },
];

type Overrides = Partial<React.ComponentProps<typeof CommandPaletteView>>;

function setup(overrides: Overrides = {}) {
  const props = {
    open: true,
    onOpenChange: vi.fn(),
    onQueryChange: vi.fn(),
    onNavigate: vi.fn(),
    navigation,
    packages: [],
    actions: [] as Action[],
    ...overrides,
  };
  render(<CommandPaletteView {...props} />);
  return props;
}

describe("CommandPaletteView", () => {
  it("открытая палитра содержит combobox с именем и подсказки клавиш", () => {
    // Arrange
    setup();

    // Act
    const input = screen.getByRole("combobox", { name: NAME });

    // Assert
    expect(input).toBeInTheDocument();
    expect(screen.getByText("выбор")).toBeInTheDocument();
    expect(screen.getByText("открыть")).toBeInTheDocument();
    expect(screen.getByText("закрыть")).toBeInTheDocument();
  });

  it("закрытая палитра ничего не рендерит", () => {
    // Arrange
    setup({ open: false });

    // Act
    const input = screen.queryByRole("combobox");

    // Assert
    expect(input).not.toBeInTheDocument();
  });

  it("стрелка вниз и Enter переходят по пункту и закрывают палитру", async () => {
    // Arrange
    const user = userEvent.setup();
    const props = setup();
    await user.click(screen.getByRole("combobox", { name: NAME }));

    // Act
    await user.keyboard("{ArrowDown}{Enter}");

    // Assert
    expect(props.onNavigate).toHaveBeenCalledWith("/packages");
    expect(props.onOpenChange).toHaveBeenCalledWith(false);
  });

  it("ввод вызывает onQueryChange и фильтрует раздел «Перейти»", async () => {
    // Arrange
    const user = userEvent.setup();
    const props = setup();

    // Act
    await user.type(screen.getByRole("combobox", { name: NAME }), "жур");

    // Assert
    expect(props.onQueryChange).toHaveBeenLastCalledWith("жур");
    expect(screen.getByRole("option", { name: "Журнал аудита" })).toBeInTheDocument();
    expect(screen.queryByRole("option", { name: NAV_PACKAGES })).not.toBeInTheDocument();
  });

  it("серверные пакеты показываются, даже если не совпадают с вводом буквально", async () => {
    // Arrange
    const user = userEvent.setup();
    setup({ packages });

    // Act
    await user.type(screen.getByRole("combobox", { name: NAME }), "lodash");

    // Assert
    const option = screen.getByRole("option", { name: LEFT_PAD });
    expect(within(option).getByText("npm")).toBeInTheDocument();
    expect(within(option).getByText("1.3.0")).toBeInTheDocument();
  });

  it("выбор пакета ведёт по его ссылке", async () => {
    // Arrange
    const user = userEvent.setup();
    const props = setup({ packages, navigation: [] });

    // Act
    await user.click(screen.getByRole("option", { name: LEFT_PAD }));

    // Assert
    expect(props.onNavigate).toHaveBeenCalledWith("/packages/left-pad");
  });

  it("действие с onSelect вызывается и закрывает палитру", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const props = setup({ actions: [{ label: "Создать токен", run: { onSelect } }] });

    // Act
    await user.click(screen.getByRole("option", { name: "Создать токен" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(props.onOpenChange).toHaveBeenCalledWith(false);
  });

  it("действие с href выполняется как переход", async () => {
    // Arrange
    const user = userEvent.setup();
    const props = setup({ actions: [{ label: "Открыть настройки", run: { href: "/settings" } }] });

    // Act
    await user.click(screen.getByRole("option", { name: "Открыть настройки" }));

    // Assert
    expect(props.onNavigate).toHaveBeenCalledWith("/settings");
  });

  it("недоступное действие не выполняется, причина видна", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    setup({
      actions: [{ label: "Снять блок", run: { onSelect }, disabledReason: "Нужна роль AppSec" }],
    });

    // Act
    await user.click(screen.getByRole("option", { name: UNBLOCK }));

    // Assert
    expect(onSelect).not.toHaveBeenCalled();
    expect(screen.getByText("Нужна роль AppSec")).toBeInTheDocument();
  });

  it("без совпадений показывает «Ничего не найдено»", async () => {
    // Arrange
    const user = userEvent.setup();
    setup();

    // Act
    await user.type(screen.getByRole("combobox", { name: NAME }), "яяя");

    // Assert
    expect(screen.getByText("Ничего не найдено")).toBeInTheDocument();
  });

  it("пока пакеты ищутся, «Ничего не найдено» не показывается", async () => {
    // Arrange
    const user = userEvent.setup();
    setup({ packagesLoading: true });

    // Act
    await user.type(screen.getByRole("combobox", { name: NAME }), "яяя");

    // Assert
    expect(screen.queryByText("Ничего не найдено")).not.toBeInTheDocument();
  });

  it("выбор пункта не отправляет его название как поисковый запрос", async () => {
    // Arrange
    const user = userEvent.setup();
    const props = setup();

    // Act
    await user.click(screen.getByRole("option", { name: "Журнал аудита" }));

    // Assert
    expect(props.onQueryChange).not.toHaveBeenCalled();
  });

  it("Esc закрывает палитру", async () => {
    // Arrange
    const user = userEvent.setup();
    const props = setup();
    await user.click(screen.getByRole("combobox", { name: NAME }));

    // Act
    await user.keyboard("{Escape}");

    // Assert
    expect(props.onOpenChange).toHaveBeenCalledWith(false);
  });

  it("действие со step-up показывает MFA и пометку для скринридера", () => {
    // Arrange
    setup({ actions: [{ label: "Отозвать токен", run: { onSelect: vi.fn() }, stepUp: true }] });

    // Act
    const option = screen.getByRole("option", { name: STEP_UP });

    // Assert
    expect(within(option).getByText("MFA")).toBeInTheDocument();
  });
});
