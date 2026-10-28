// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовые компоненты-помощники не экспортируются
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import {
  Popover,
  PopoverContent,
  PopoverDescription,
  PopoverTitle,
  PopoverTrigger,
} from "~/shared/ui/primitives";

function Example() {
  return (
    <Popover>
      <PopoverTrigger>{"Уведомления"}</PopoverTrigger>
      <PopoverContent>
        <PopoverTitle>{"Центр уведомлений"}</PopoverTitle>
        <PopoverDescription>{"Новых событий нет"}</PopoverDescription>
      </PopoverContent>
    </Popover>
  );
}

describe("Popover", () => {
  it("открывается по клику и показывает содержимое", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Example />);

    // Act
    await user.click(screen.getByRole("button", { name: "Уведомления" }));

    // Assert
    expect(await screen.findByText("Новых событий нет")).toBeInTheDocument();
  });

  it("закрывается по Esc", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Example />);
    await user.click(screen.getByRole("button", { name: "Уведомления" }));
    await screen.findByText("Новых событий нет");

    // Act
    await user.keyboard("{Escape}");

    // Assert
    await waitFor(() => expect(screen.queryByText("Новых событий нет")).not.toBeInTheDocument());
  });
});
