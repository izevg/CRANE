// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовые компоненты-помощники не экспортируются
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import {
  Dialog,
  DialogClose,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "~/shared/ui/primitives";

function Example() {
  return (
    <Dialog>
      <DialogTrigger>{"Открыть"}</DialogTrigger>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>{"Новый токен"}</DialogTitle>
          <DialogDescription>{"Срок действия до 90 дней"}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <DialogClose>{"Отмена"}</DialogClose>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

describe("Dialog", () => {
  it("открывается по триггеру, получает имя из заголовка и переносит фокус внутрь", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Example />);

    // Act
    await user.click(screen.getByRole("button", { name: "Открыть" }));

    // Assert
    const dialog = await screen.findByRole("dialog", { name: "Новый токен" });
    expect(dialog).toHaveAccessibleDescription("Срок действия до 90 дней");
    await waitFor(() => expect(dialog).toContainElement(document.activeElement as HTMLElement));
  });

  it("закрывается по Esc", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Example />);
    await user.click(screen.getByRole("button", { name: "Открыть" }));
    await screen.findByRole("dialog");

    // Act
    await user.keyboard("{Escape}");

    // Assert
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("кнопка «Закрыть» закрывает диалог", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Example />);
    await user.click(screen.getByRole("button", { name: "Открыть" }));

    // Act
    await user.click(await screen.findByRole("button", { name: "Закрыть" }));

    // Assert
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });
});
