// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовые компоненты-помощники не экспортируются
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useRef } from "react";
import { describe, expect, it, vi } from "vitest";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "~/shared/ui/primitives";

function Example({ onConfirm }: { onConfirm: () => void }) {
  const cancelRef = useRef<HTMLButtonElement>(null);
  return (
    <AlertDialog>
      <AlertDialogTrigger>{"Удалить"}</AlertDialogTrigger>
      <AlertDialogContent initialFocus={cancelRef}>
        <AlertDialogHeader>
          <AlertDialogTitle>{"Удалить пакет?"}</AlertDialogTitle>
          <AlertDialogDescription>{"Версии станут недоступны"}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogAction variant="destructive" onClick={onConfirm}>
            {"Удалить"}
          </AlertDialogAction>
          <AlertDialogCancel ref={cancelRef}>{"Отмена"}</AlertDialogCancel>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}

describe("AlertDialog", () => {
  it("открывается с ролью alertdialog и фокусом на «Отмена» через initialFocus", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Example onConfirm={vi.fn()} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Удалить" }));

    // Assert
    expect(await screen.findByRole("alertdialog", { name: "Удалить пакет?" })).toBeInTheDocument();
    await waitFor(() => expect(screen.getByRole("button", { name: "Отмена" })).toHaveFocus());
  });

  it("подтверждение вызывает обработчик и закрывает диалог", async () => {
    // Arrange
    const user = userEvent.setup();
    const onConfirm = vi.fn();
    render(<Example onConfirm={onConfirm} />);
    await user.click(screen.getByRole("button", { name: "Удалить" }));
    const dialog = await screen.findByRole("alertdialog");

    // Act
    await user.click(within(dialog).getByRole("button", { name: "Удалить" }));

    // Assert
    expect(onConfirm).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.queryByRole("alertdialog")).not.toBeInTheDocument());
  });

  it("клик вне диалога его не закрывает", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Example onConfirm={vi.fn()} />);
    await user.click(screen.getByRole("button", { name: "Удалить" }));
    await screen.findByRole("alertdialog");

    // Act
    await user.click(document.body);

    // Assert
    expect(screen.getByRole("alertdialog")).toBeInTheDocument();
  });
});
