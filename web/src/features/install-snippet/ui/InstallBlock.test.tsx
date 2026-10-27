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
  client: "npm" as const,
  available_clients: ["npm", "pnpm", "yarn"] as InstallClient[],
  setup_steps: [{ title: "Вход", code: "cctl login\ncctl setup npm" }],
  manual_config: {
    path: ".npmrc",
    code: `//crane.example/npm/:_authToken=${TOKEN_REF}`,
  },
  install: { code: "npm i @acme/ui@1.4.2" },
};

const COPY_STEP = /^Копировать/;
const KEY = "crane.install.npm";

/** Страница-обёртка: список клиентов берёт из ответа сервера и отдаёт хуку, блок управляемый. */
function Page({
  onClientChange,
  answered = true,
}: {
  onClientChange: (client: InstallClient) => void;
  /** `false` — первый рендер до ответа сервера: инструкций и списка клиентов ещё нет. */
  answered?: boolean;
}) {
  const instructions = answered ? INSTRUCTIONS : undefined;
  const [client, setClient] = useInstallClient("npm", instructions?.available_clients);
  const change = useCallback(
    (next: InstallClient) => {
      setClient(next);
      onClientChange(next);
    },
    [setClient, onClientChange],
  );
  return (
    <InstallBlock
      client={client}
      onClientChange={change}
      instructions={instructions}
      envHref="/help/env"
    />
  );
}

function renderBlock(onClientChange = vi.fn()) {
  return render(<Page onClientChange={onClientChange} />);
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

  it("клиент из хранилища вне available_clients отбрасывается в пользу первого доступного", () => {
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

  it("до первого ответа сервера команд для копирования нет, шаги помечены занятыми", () => {
    // Arrange
    // Act
    const { container } = render(<Page onClientChange={vi.fn()} answered={false} />);

    // Assert
    expect(screen.queryByRole("button", { name: COPY_STEP })).not.toBeInTheDocument();
    expect(container.querySelectorAll("[aria-busy=true]")).toHaveLength(2);
  });

  it("инструкции не для выбранного клиента скрывают команды чужого клиента", () => {
    // Arrange
    localStorage.setItem(KEY, "pnpm");

    // Act
    renderBlock();

    // Assert
    expect(pressed("pnpm")).toHaveAttribute("aria-pressed", "true");
    expect(screen.queryByRole("button", { name: COPY_STEP })).not.toBeInTheDocument();
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
