import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Action } from "~/shared/ui/actions";
import { ProblemState, type ProblemView } from "~/shared/ui/status";

const REQUEST_ID = "a88d0c1e-5b7a-4f3e-9d21-6973c0ffee42";
const RULE_LABEL = /Правило/;
const CONTACT_LABEL = /Контакт/;
const REQUEST_ACCESS_LABEL = /Запросить доступ/;
const REQUEST_ID_LABEL = /Идентификатор запроса/;
const HOME: Action = { label: "На главную", run: { href: "/" } };

describe("ProblemState", () => {
  it("not-found показывает нейтральный текст, поиск и «На главную» без правила, контакта и доступа", () => {
    // Arrange
    const problem: ProblemView = {
      kind: "not-found",
      requestId: REQUEST_ID,
      search: <input aria-label="Поиск пакета" />,
      home: HOME,
    };

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("heading", { name: "Не найдено или нет доступа" })).toBeInTheDocument();
    expect(screen.getByText("Проверьте адрес или имя пакета")).toBeInTheDocument();
    expect(screen.getByRole("textbox", { name: "Поиск пакета" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "На главную" })).toHaveAttribute("href", "/");
    expect(screen.queryByText(RULE_LABEL)).not.toBeInTheDocument();
    expect(screen.queryByText(CONTACT_LABEL)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: REQUEST_ACCESS_LABEL })).not.toBeInTheDocument();
    expect(screen.queryByRole("list")).not.toBeInTheDocument();
  });

  it("not-found без requestId не рисует идентификатор запроса", () => {
    // Arrange
    const problem = { kind: "not-found", home: HOME } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.queryByText(REQUEST_ID_LABEL)).not.toBeInTheDocument();
  });

  it("forbidden показывает правило, request_id с полным значением, контакт и шаги списком", () => {
    // Arrange
    const problem = {
      kind: "forbidden",
      ruleId: "policy.deny.unlicensed",
      requestId: REQUEST_ID,
      remediation: ["Запросите роль AppSec", "Повторите операцию"],
      contact: "appsec@example.org",
    } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("heading", { name: "Недостаточно прав" })).toBeInTheDocument();
    expect(screen.getByText("Правило: policy.deny.unlicensed")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: `Скопировать идентификатор запроса: ${REQUEST_ID}` }),
    ).toBeInTheDocument();
    expect(screen.getByText("Контакт: appsec@example.org")).toBeInTheDocument();
    const steps = within(screen.getByRole("list")).getAllByRole("listitem");
    expect(steps.map((step) => step.textContent)).toEqual([
      "Запросите роль AppSec",
      "Повторите операцию",
    ]);
  });

  it("forbidden без requestAccess не рисует «Запросить доступ»", () => {
    // Arrange
    const problem = {
      kind: "forbidden",
      ruleId: "policy.deny.unlicensed",
      requestId: REQUEST_ID,
      remediation: [],
    } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.queryByRole("button", { name: "Запросить доступ" })).not.toBeInTheDocument();
  });

  it("forbidden с requestAccess показывает кнопку, и она вызывает обработчик", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <ProblemState
        problem={{
          kind: "forbidden",
          ruleId: "policy.deny.unlicensed",
          requestId: REQUEST_ID,
          remediation: [],
          requestAccess: { label: "Запросить доступ", run: { onSelect } },
        }}
      />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Запросить доступ" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("forbidden с пустым remediation не рисует список", () => {
    // Arrange
    const problem = {
      kind: "forbidden",
      ruleId: "policy.deny.unlicensed",
      requestId: REQUEST_ID,
      remediation: [],
    } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.queryByRole("list")).not.toBeInTheDocument();
  });

  it("error показывает заголовок, подробности, правило, request_id и шаги", () => {
    // Arrange
    const problem = {
      kind: "error",
      title: "Файл слишком большой",
      detail: "Лимит артефакта — 5 ГБ",
      ruleId: "limits.artifact.size",
      requestId: REQUEST_ID,
      remediation: ["Разбейте архив"],
    } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Файл слишком большой" })).toBeInTheDocument();
    expect(screen.getByText("Лимит артефакта — 5 ГБ")).toBeInTheDocument();
    expect(screen.getByText("Правило: limits.artifact.size")).toBeInTheDocument();
    expect(screen.getByRole("listitem")).toHaveTextContent("Разбейте архив");
  });

  it("error без retry не рисует «Повторить»", () => {
    // Arrange
    const problem = {
      kind: "error",
      title: "Сбой",
      requestId: REQUEST_ID,
      remediation: [],
    } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.queryByRole("button", { name: "Повторить" })).not.toBeInTheDocument();
  });

  it("error с retry показывает «Повторить», и клик вызывает обработчик", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <ProblemState
        problem={{
          kind: "error",
          title: "Сбой",
          requestId: REQUEST_ID,
          remediation: [],
          retry: { label: "Повторить", run: { onSelect } },
        }}
      />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Повторить" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("error с diagnostics показывает ссылку «Диагностика»", () => {
    // Arrange
    const problem = {
      kind: "error",
      title: "Сбой",
      requestId: REQUEST_ID,
      remediation: [],
      diagnostics: { label: "Диагностика", run: { href: "/diagnostics" } },
    } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("link", { name: "Диагностика" })).toHaveAttribute(
      "href",
      "/diagnostics",
    );
  });

  it("offline показывает «Нет соединения» и «Повторить» только при переданном действии", () => {
    // Arrange
    const problem = { kind: "offline" } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("heading", { name: "Нет соединения" })).toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("session показывает «Сессия завершена» и «Войти снова»", () => {
    // Arrange
    const problem = {
      kind: "session",
      signIn: { label: "Войти снова", run: { href: "/login" } },
    } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("heading", { name: "Сессия завершена" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "Войти снова" })).toHaveAttribute("href", "/login");
  });

  it("заголовок получает уровень из headingLevel", () => {
    // Arrange
    const problem = { kind: "offline" } as const;

    // Act
    render(<ProblemState problem={problem} headingLevel={3} />);

    // Assert
    expect(screen.getByRole("heading", { level: 3, name: "Нет соединения" })).toBeInTheDocument();
  });

  it("по умолчанию заголовок второго уровня", () => {
    // Arrange
    const problem = { kind: "offline" } as const;

    // Act
    render(<ProblemState problem={problem} />);

    // Assert
    expect(screen.getByRole("heading", { level: 2 })).toBeInTheDocument();
  });
});
