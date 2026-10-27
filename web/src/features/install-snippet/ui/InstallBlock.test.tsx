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

/** Страница-обёртка: предпочтение берёт у хука, «сервер» отвечает для переданного клиента, блок управляемый. */
function Page({
  onClientChange,
  answeredClient = "npm",
  loading = false,
}: {
  onClientChange: (client: InstallClient) => void;
  /** Клиент, для которого «сервер» дал инструкции; `null` — ответа ещё нет. */
  answeredClient?: InstallClient | null;
  loading?: boolean;
}) {
  const [preferred, setPreferred] = useInstallClient("npm");
  const change = useCallback(
    (next: InstallClient) => {
      setPreferred(next);
      onClientChange(next);
    },
    [setPreferred, onClientChange],
  );
  const instructions =
    answeredClient === null ? undefined : { ...INSTRUCTIONS, client: answeredClient };
  return (
    <InstallBlock
      preferred={preferred}
      onClientChange={change}
      instructions={instructions}
      loading={loading}
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
  it("без предпочтения выделен клиент, которого дал сервер", () => {
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
    render(<Page onClientChange={vi.fn()} answeredClient="yarn" />);

    // Assert
    expect(pressed("yarn")).toHaveAttribute("aria-pressed", "true");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("мусор в хранилище не принимается за предпочтение: заметки о подмене нет", () => {
    // Arrange
    localStorage.setItem(KEY, "rm -rf");

    // Act
    renderBlock();

    // Assert
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(pressed("npm")).toHaveAttribute("aria-pressed", "true");
  });

  it("сохранённый доступный клиент подхватывается из хранилища", () => {
    // Arrange
    localStorage.setItem(KEY, "pnpm");

    // Act
    render(<Page onClientChange={vi.fn()} answeredClient="pnpm" />);

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
    const { container } = render(<Page onClientChange={vi.fn()} answeredClient={null} />);

    // Assert
    expect(screen.queryByRole("button", { name: COPY_STEP })).not.toBeInTheDocument();
    expect(container.querySelectorAll("[aria-busy=true]")).toHaveLength(2);
  });

  it("пока идёт запрос, команды прежнего клиента скрыты", () => {
    // Arrange
    // Act
    render(<Page onClientChange={vi.fn()} loading />);

    // Assert
    expect(screen.queryByRole("button", { name: COPY_STEP })).not.toBeInTheDocument();
  });

  it("если сервер дал не предпочитаемого клиента, заметка называет обоих", () => {
    // Arrange
    localStorage.setItem(KEY, "pnpm");

    // Act
    renderBlock();

    // Assert
    expect(screen.getByRole("status")).toHaveTextContent(
      "pnpm недоступен для этого репозитория, показаны команды для npm",
    );
  });

  it("пока идёт запрос, расхождение не объявляется подменой", () => {
    // Arrange
    localStorage.setItem(KEY, "pnpm");

    // Act
    render(<Page onClientChange={vi.fn()} loading />);

    // Assert
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
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
