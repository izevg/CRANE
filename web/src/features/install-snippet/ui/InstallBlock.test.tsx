import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { CLIENTS_BY_FORMAT, InstallBlock } from "../index";

// Ссылка на переменную окружения собирается из частей, чтобы линтер не принял её за шаблонную строку.
const TOKEN_REF = ["$", "{CRANE_TOKEN}"].join("");

const INSTRUCTIONS = {
  client: "npm" as const,
  setup_steps: [{ title: "Вход", code: "cctl login\ncctl setup npm" }],
  manual_config: {
    path: ".npmrc",
    code: `//crane.example/npm/:_authToken=${TOKEN_REF}`,
  },
  install: { code: "npm i @acme/ui@1.4.2" },
};

const KEY = "crane.install.npm";

function renderBlock(onClientChange = vi.fn()) {
  return render(
    <InstallBlock
      format="npm"
      clients={CLIENTS_BY_FORMAT.npm}
      instructions={INSTRUCTIONS}
      onClientChange={onClientChange}
      envHref="/help/env"
    />,
  );
}

const pressed = (name: string) => screen.getByRole("button", { name });

afterEach(() => {
  localStorage.clear();
});

describe("InstallBlock", () => {
  it("по умолчанию выбран первый клиент формата", () => {
    // Arrange
    // Act
    renderBlock();

    // Assert
    expect(pressed("npm")).toHaveAttribute("aria-pressed", "true");
    expect(pressed("pnpm")).toHaveAttribute("aria-pressed", "false");
  });

  it("смена клиента вызывает onClientChange с новым клиентом", async () => {
    // Arrange
    const user = userEvent.setup();
    const onClientChange = vi.fn();
    renderBlock(onClientChange);

    // Act
    await user.click(pressed("pnpm"));

    // Assert
    expect(onClientChange).toHaveBeenCalledWith("pnpm");
  });

  it("сохранённый клиент, отличный от показанной инструкции, запрашивает её у страницы", () => {
    // Arrange
    localStorage.setItem(KEY, "yarn");
    const onClientChange = vi.fn();

    // Act
    renderBlock(onClientChange);

    // Assert
    expect(onClientChange).toHaveBeenCalledWith("yarn");
  });

  it("выбор запоминается: новый рендер того же формата начинает с него", async () => {
    // Arrange
    const user = userEvent.setup();
    const first = renderBlock();
    await user.click(pressed("yarn"));
    first.unmount();

    // Act
    renderBlock();

    // Assert
    expect(pressed("yarn")).toHaveAttribute("aria-pressed", "true");
  });

  it("невалидное значение в хранилище заменяется первым клиентом", () => {
    // Arrange
    localStorage.setItem(KEY, "poetry");

    // Act
    renderBlock();

    // Assert
    expect(pressed("npm")).toHaveAttribute("aria-pressed", "true");
  });

  it("сохранённый доступный клиент подхватывается из хранилища", () => {
    // Arrange
    localStorage.setItem(KEY, "pnpm");

    // Act
    renderBlock();

    // Assert
    expect(pressed("pnpm")).toHaveAttribute("aria-pressed", "true");
  });

  it("ручная настройка свёрнута по умолчанию и раскрывается", async () => {
    // Arrange
    const user = userEvent.setup();
    renderBlock();
    const toggle = screen.getByRole("button", { name: "Настроить вручную (.npmrc)" });
    const before = toggle.getAttribute("aria-expanded");

    // Act
    await user.click(toggle);

    // Assert
    expect([before, toggle.getAttribute("aria-expanded")]).toEqual(["false", "true"]);
  });

  it("кнопки копирования имеют доступные имена из подписей блока", () => {
    // Arrange
    // Act
    renderBlock();

    // Assert
    expect(
      screen.getByRole("button", { name: "Копировать команду настройки" }),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Копировать команду установки" }),
    ).toBeInTheDocument();
  });
});
