// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовая страница-обёртка не экспортируется
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useCallback } from "react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { InstallClient } from "~/shared/api";
import { InstallBlock, useInstallClient } from "../index";

// Ссылка на переменную окружения собирается из частей, чтобы линтер не принял её за шаблонную строку.
const TOKEN_REF = ["$", "{CRANE_TOKEN}"].join("");

const INSTRUCTIONS = {
  setup_steps: [{ title: "Вход", code: "cctl login\ncctl setup npm" }],
  manual_config: {
    path: ".npmrc",
    code: `//crane.example/npm/:_authToken=${TOKEN_REF}`,
  },
  install: { code: "npm i @acme/ui@1.4.2" },
};

const COPY_STEP = /^Копировать/;
const KEY = "crane.install.npm";

/** Страница-обёртка: клиента держит хук страницы, блок управляемый. */
function Page({
  onClientChange,
  loading = false,
}: {
  onClientChange: (client: InstallClient) => void;
  loading?: boolean;
}) {
  const [client, setClient] = useInstallClient("npm");
  const change = useCallback(
    (next: InstallClient) => {
      setClient(next);
      onClientChange(next);
    },
    [setClient, onClientChange],
  );
  return (
    <InstallBlock
      format="npm"
      client={client}
      onClientChange={change}
      instructions={INSTRUCTIONS}
      loading={loading}
      envHref="/help/env"
    />
  );
}

function renderBlock(onClientChange = vi.fn(), loading = false) {
  return render(<Page onClientChange={onClientChange} loading={loading} />);
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

  it("пока инструкции грузятся, команд для копирования нет и шаги помечены занятыми", () => {
    // Arrange
    // Act
    const { container } = renderBlock(vi.fn(), true);

    // Assert
    expect(screen.queryByRole("button", { name: COPY_STEP })).not.toBeInTheDocument();
    expect(container.querySelectorAll("[aria-busy=true]")).toHaveLength(2);
  });

  it("кнопка копирования шага настройки называется по заголовку шага", () => {
    // Arrange
    // Act
    renderBlock();

    // Assert
    expect(screen.getByRole("button", { name: "Копировать: Вход" })).toBeInTheDocument();
  });

  it("кнопка копирования установки имеет доступное имя", () => {
    // Arrange
    // Act
    renderBlock();

    // Assert
    expect(
      screen.getByRole("button", { name: "Копировать команду установки" }),
    ).toBeInTheDocument();
  });
});
