import { render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { LiveRegion } from "~/shared/lib/a11y";
import { type ProbeCheck, ProbeReport } from "~/shared/ui/data-display";

const CHECKS: ProbeCheck[] = [
  { id: "reachable", label: "Хранилище доступно", status: "pass" },
  { id: "free-space", label: "Свободное место", status: "warn", detail: "Осталось 3 ГиБ" },
  { id: "write", label: "Запись в каталог", status: "fail", detail: "permission denied" },
  { id: "oidc", label: "Единый вход", status: "skipped" },
];

const EXPECTED_ROWS = 4;
const SUMMARY = "Проверка завершена: 1 успешно, 1 с предупреждением, 1 с ошибкой.";

function report(state: Parameters<typeof ProbeReport>[0]["state"], expected?: number) {
  return (
    <>
      <LiveRegion />
      <ProbeReport state={state} {...(expected === undefined ? {} : { expected })} />
    </>
  );
}

describe("ProbeReport", () => {
  it("в покое ничего не показывает", () => {
    // Arrange
    const state = "idle";

    // Act
    render(report(state));

    // Assert
    expect(screen.queryByRole("list")).toBeNull();
    expect(screen.queryByText("Проверяем связь…")).toBeNull();
  });

  it("пока идёт проверка, показывает текст и помечает его занятым", () => {
    // Arrange
    const state = "running";

    // Act
    render(report(state));

    // Assert
    expect(screen.getByText("Проверяем связь…")).toHaveAttribute("aria-busy", "true");
  });

  it("при running с expected резервируется столько строк-заглушек", () => {
    // Arrange
    const state = "running";

    // Act
    const { container } = render(report(state, EXPECTED_ROWS));

    // Assert
    expect(container.querySelectorAll("[data-slot=skeleton]")).toHaveLength(EXPECTED_ROWS);
  });

  it("у каждой проверки есть название, статус текстом и пояснение", () => {
    // Arrange
    const state = { checks: CHECKS };

    // Act
    render(report(state));

    // Assert
    const rows = screen.getAllByRole("listitem");
    expect(rows).toHaveLength(CHECKS.length);
    expect(rows[1]).toHaveTextContent("Свободное место · предупреждение");
    expect(rows[1]).toHaveTextContent("Осталось 3 ГиБ");
    expect(rows[2]).toHaveTextContent("Запись в каталог · ошибка");
    expect(rows[3]).toHaveTextContent("Единый вход · пропущено");
  });

  it("показывает сводку видимым текстом", () => {
    // Arrange
    const state = { checks: CHECKS };

    // Act
    render(report(state));

    // Assert
    expect(screen.getByText(SUMMARY)).toBeVisible();
  });

  it("сводка склоняется по числу: два предупреждения и две ошибки", () => {
    // Arrange
    const state = {
      checks: [
        { id: "a", label: "A", status: "warn" as const },
        { id: "b", label: "B", status: "warn" as const },
        { id: "c", label: "C", status: "fail" as const },
        { id: "d", label: "D", status: "fail" as const },
      ],
    };

    // Act
    render(report(state));

    // Assert
    expect(
      screen.getByText("Проверка завершена: 0 успешно, 2 с предупреждениями, 2 с ошибками."),
    ).toBeVisible();
  });

  it("по завершении объявляет одну сводку, а не строки", async () => {
    // Arrange
    const state = { checks: CHECKS };

    // Act
    render(report(state));

    // Assert
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent(SUMMARY));
  });

  it("при отказе самой проверки показывает причину в alert", () => {
    // Arrange
    const state = { error: "Сервер не ответил" };

    // Act
    render(report(state));

    // Assert
    expect(screen.getByRole("alert")).toHaveTextContent("Сервер не ответил");
  });
});
