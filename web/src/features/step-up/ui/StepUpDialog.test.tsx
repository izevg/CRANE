import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { StepUpDialog } from "../index";

function open(onContinue = vi.fn()) {
  render(
    <StepUpDialog
      open
      onOpenChange={vi.fn()}
      actionLabel="Раскрыть данные события"
      onContinue={onContinue}
    />,
  );
}

describe("StepUpDialog", () => {
  it("называет действие и объясняет возврат с сохранённой формой", () => {
    // Arrange
    // Act
    open();

    // Assert
    expect(
      screen.getByRole("alertdialog", { name: "Подтвердите вход" }),
    ).toHaveAccessibleDescription(
      "Действие «Раскрыть данные события» требует подтверждения вторым фактором",
    );
    expect(
      screen.getByText("После подтверждения вы вернётесь к действию, данные формы сохранятся"),
    ).toBeVisible();
  });

  it("пояснение — абзац-подпись, а не список из одного пункта", () => {
    // Arrange
    // Act
    open();

    // Assert
    expect(screen.queryByRole("list")).not.toBeInTheDocument();
  });

  it("начальный фокус на «Продолжить в Keycloak», а не на «Отмена»", async () => {
    // Arrange
    // Act
    open();

    // Assert
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Продолжить в Keycloak" })).toHaveFocus(),
    );
  });

  it("«Продолжить в Keycloak» вызывает onContinue", async () => {
    // Arrange
    const user = userEvent.setup();
    const onContinue = vi.fn();
    open(onContinue);

    // Act
    await user.click(screen.getByRole("button", { name: "Продолжить в Keycloak" }));

    // Assert
    expect(onContinue).toHaveBeenCalledTimes(1);
  });

  it("своих полей для кода нет", () => {
    // Arrange
    // Act
    open();

    // Assert
    expect(screen.queryByRole("textbox")).not.toBeInTheDocument();
  });
});
