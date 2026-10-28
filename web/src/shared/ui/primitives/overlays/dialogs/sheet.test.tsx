// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовые компоненты-помощники не экспортируются
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import {
  Sheet,
  SheetBody,
  SheetContent,
  SheetDescription,
  SheetFooter,
  SheetHeader,
  SheetTitle,
  SheetTrigger,
} from "~/shared/ui/primitives";

function Example() {
  return (
    <Sheet>
      <SheetTrigger>{"Детали"}</SheetTrigger>
      <SheetContent side="left">
        <SheetHeader>
          <SheetTitle>{"Событие"}</SheetTitle>
          <SheetDescription>{"Сведения о событии журнала"}</SheetDescription>
        </SheetHeader>
        <SheetBody>{"Тело события"}</SheetBody>
        <SheetFooter>{"Подвал"}</SheetFooter>
      </SheetContent>
    </Sheet>
  );
}

describe("Sheet", () => {
  it("открывается как диалог с именем из заголовка", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Example />);

    // Act
    await user.click(screen.getByRole("button", { name: "Детали" }));

    // Assert
    expect(await screen.findByRole("dialog", { name: "Событие" })).toBeInTheDocument();
  });

  it("закрывается по Esc", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Example />);
    await user.click(screen.getByRole("button", { name: "Детали" }));
    await screen.findByRole("dialog");

    // Act
    await user.keyboard("{Escape}");

    // Assert
    await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
  });

  it("тело листа лежит внутри диалога", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Example />);

    // Act
    await user.click(screen.getByRole("button", { name: "Детали" }));

    // Assert
    expect(
      await within(await screen.findByRole("dialog")).findByText("Тело события"),
    ).toBeInTheDocument();
  });
});
