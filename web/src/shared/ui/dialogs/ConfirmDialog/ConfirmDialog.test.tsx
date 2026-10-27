import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import type { Action } from "~/shared/ui/actions";
import { ConfirmDialog } from "~/shared/ui/dialogs";

const destructive: Action = {
  label: "Удалить",
  run: { onSelect: vi.fn() },
  destructive: true,
};
const regular: Action = { label: "Продлить", run: { onSelect: vi.fn() } };

describe("ConfirmDialog", () => {
  it("деструктивное действие: начальный фокус на «Отмена»", async () => {
    // Arrange
    // Act
    render(
      <ConfirmDialog
        open
        onOpenChange={vi.fn()}
        title="Удалить версию 1.4.2?"
        confirm={destructive}
      />,
    );

    // Assert
    await waitFor(() => expect(screen.getByRole("button", { name: "Отмена" })).toHaveFocus());
  });

  it("обычное действие с полем: фокус на поле, а не на «Отмена»", async () => {
    // Arrange
    // Act
    render(
      <ConfirmDialog open onOpenChange={vi.fn()} title="Продлить срок?" confirm={regular}>
        <input aria-label="Причина" />
      </ConfirmDialog>,
    );

    // Assert
    await waitFor(() => expect(screen.getByRole("textbox", { name: "Причина" })).toHaveFocus());
    expect(screen.getByRole("button", { name: "Отмена" })).not.toHaveFocus();
  });

  it("обычное действие без полей: фокус на кнопке подтверждения", async () => {
    // Arrange
    // Act
    render(<ConfirmDialog open onOpenChange={vi.fn()} title="Продлить срок?" confirm={regular} />);

    // Assert
    await waitFor(() => expect(screen.getByRole("button", { name: "Продлить" })).toHaveFocus());
  });

  it("последствия выводятся списком", () => {
    // Arrange
    const consequences = ["Версия перестанет скачиваться", "Владельцы получат уведомление"];

    // Act
    render(
      <ConfirmDialog
        open
        onOpenChange={vi.fn()}
        title="Удалить версию 1.4.2?"
        consequences={consequences}
        confirm={destructive}
      />,
    );

    // Assert
    expect(screen.getAllByRole("listitem").map((item) => item.textContent)).toEqual(consequences);
  });

  it("step-up действие показывает бейдж MFA", () => {
    // Arrange
    const confirm: Action = { ...destructive, stepUp: true };

    // Act
    render(<ConfirmDialog open onOpenChange={vi.fn()} title="Заблокировать?" confirm={confirm} />);

    // Assert
    expect(screen.getByText("MFA")).toBeVisible();
  });

  it("«Отмена» просит закрыть диалог", async () => {
    // Arrange
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(
      <ConfirmDialog open onOpenChange={onOpenChange} title="Удалить?" confirm={destructive} />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Отмена" }));

    // Assert
    expect(onOpenChange).toHaveBeenCalledWith(false);
  });

  it("подтверждение вызывает обработчик действия", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <ConfirmDialog
        open
        onOpenChange={vi.fn()}
        title="Удалить?"
        confirm={{ ...destructive, run: { onSelect } }}
      />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Удалить" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("во время выполнения «Отмена» объясняет причину и не закрывает диалог", async () => {
    // Arrange
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(
      <ConfirmDialog
        open
        onOpenChange={onOpenChange}
        title="Удалить?"
        confirm={destructive}
        pending
      />,
    );
    const cancel = screen.getByRole("button", { name: "Отмена" });

    // Act
    await user.click(cancel);

    // Assert
    expect(onOpenChange).not.toHaveBeenCalled();
    expect(cancel).toHaveAccessibleDescription("Выполняется…");
  });

  it("во время выполнения подтверждение недоступно и Esc не закрывает диалог", async () => {
    // Arrange
    const user = userEvent.setup();
    const onOpenChange = vi.fn();
    render(
      <ConfirmDialog
        open
        onOpenChange={onOpenChange}
        title="Удалить?"
        confirm={destructive}
        pending
      />,
    );

    // Act
    await user.keyboard("{Escape}");

    // Assert
    expect(onOpenChange).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: "Удалить" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
  });
});
