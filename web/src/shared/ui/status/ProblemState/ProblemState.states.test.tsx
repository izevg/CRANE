import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { ProblemState } from "~/shared/ui/status";

const AUTO_RETRY = /Повторим попытку автоматически/;
const LAST_DATA = /Последние данные/;
const REQUEST_ID_LABEL = /Идентификатор запроса/;

describe("ProblemState: сессия, сбой интерфейса, сеть, заглушенный раздел", () => {
  it("session с причиной показывает «Сессия завершена», подсказку по причине и «Войти снова»", () => {
    // Arrange
    const problem = {
      kind: "session",
      reason: "session_expired",
      signIn: { label: "Войти снова", run: { href: "/login" } },
    } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("heading", { name: "Сессия завершена" })).toBeInTheDocument();
    expect(
      screen.getByText("Время сессии истекло. Войдите снова, чтобы продолжить работу."),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Войти снова" })).toHaveAttribute("href", "/login");
  });

  it.each([
    ["session_revoked", "Сессия отозвана. Войдите снова, чтобы продолжить работу."],
    [
      "session_elsewhere",
      "Вы вошли на другом устройстве, и эта сессия завершена. Войдите снова, чтобы продолжить работу.",
    ],
  ] as const)("session %s: подсказка своя", (reason, hint) => {
    // Arrange
    const problem = {
      kind: "session",
      reason,
      signIn: { label: "Войти", run: { href: "/login" } },
    } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByText(hint)).toBeInTheDocument();
  });

  it("session без причины: «Войдите, чтобы продолжить» вместо «Сессия завершена»", () => {
    // Arrange
    const problem = {
      kind: "session",
      signIn: { label: "Войти", run: { href: "/login" } },
    } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("heading", { name: "Войдите, чтобы продолжить" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Сессия завершена" })).not.toBeInTheDocument();
  });

  it("session без signIn: кнопки нет, подсказка называет версию, в которой вход появится", () => {
    // Arrange
    const problem = { kind: "session", availableIn: "v0.4.0" } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("heading", { name: "Войдите, чтобы продолжить" })).toBeInTheDocument();
    expect(screen.getByText("Вход в веб-консоль появится в CRANE 0.4.0.")).toBeInTheDocument();
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("client chunk: «Не удалось загрузить раздел», подсказка об обновлении и «Обновить страницу»", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <ProblemState
        problem={{
          kind: "client",
          cause: "chunk",
          reload: { label: "Обновить страницу", run: { onSelect } },
        }}
      />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Обновить страницу" }));

    // Assert
    expect(
      screen.getByRole("heading", { name: "Не удалось загрузить раздел" }),
    ).toBeInTheDocument();
    expect(screen.getByText("Возможно, CRANE обновился. Обновите страницу.")).toBeInTheDocument();
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("client render: «Не удалось открыть страницу», без идентификатора запроса и текста исключения", () => {
    // Arrange
    const problem = { kind: "client", cause: "render" } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(
      screen.getByRole("heading", { name: "Не удалось открыть страницу" }),
    ).toBeInTheDocument();
    expect(screen.queryByText(REQUEST_ID_LABEL)).not.toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("offline по умолчанию — «Нет соединения» (нет сети у пользователя)", () => {
    // Arrange
    const problem = { kind: "offline" } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("heading", { name: "Нет соединения" })).toBeInTheDocument();
  });

  it("offline с причиной server — «Сервер CRANE недоступен»", () => {
    // Arrange
    const problem = {
      kind: "offline",
      cause: "server",
      retry: { label: "Повторить", run: { href: "/" } },
    } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("heading", { name: "Сервер CRANE не отвечает" })).toBeInTheDocument();
    expect(screen.queryByRole("heading", { name: "Нет соединения" })).not.toBeInTheDocument();
  });

  it("planned: «Раздел пока недоступен», версия без префикса v в подсказке, без действий", () => {
    // Arrange
    const problem = { kind: "planned", replacedIn: "v0.4.0" } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("heading", { name: "Раздел пока недоступен" })).toBeInTheDocument();
    expect(screen.getByText("Он появится в CRANE 0.4.0.")).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
  });

  it("session без signIn игнорирует причину: показывает вариант первого входа", () => {
    // Arrange
    const problem = { kind: "session", reason: "session_revoked", availableIn: "v0.4.0" } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.queryByRole("heading", { name: "Сессия завершена" })).not.toBeInTheDocument();
    expect(screen.getByText("Вход в веб-консоль появится в CRANE 0.4.0.")).toBeInTheDocument();
  });

  it("planned с областью «веб-консоль»: заголовок про консоль целиком", () => {
    // Arrange
    const problem = { kind: "planned", replacedIn: "v0.4.0", scope: "console" } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(
      screen.getByRole("heading", { name: "Веб-консоль появится в CRANE 0.4.0" }),
    ).toBeInTheDocument();
  });

  it("offline network: подсказка про сеть", () => {
    // Arrange
    const problem = { kind: "offline", cause: "network" } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(
      screen.getByText("Проверьте сеть — данные обновятся, когда связь вернётся."),
    ).toBeInTheDocument();
  });

  it("offline server: обещает автоматический повтор, а не «последние данные»", () => {
    // Arrange
    const problem = { kind: "offline", cause: "server" } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByText(AUTO_RETRY)).toBeInTheDocument();
    expect(screen.queryByText(LAST_DATA)).not.toBeInTheDocument();
  });
});
