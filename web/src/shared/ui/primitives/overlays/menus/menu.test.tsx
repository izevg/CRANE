// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовые компоненты-помощники не экспортируются
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import {
  Menu,
  MenuContent,
  MenuGroup,
  MenuItem,
  MenuLabel,
  MenuSeparator,
  MenuTrigger,
} from "~/shared/ui/primitives";

function Example({ onSelect }: { onSelect: () => void }) {
  return (
    <Menu>
      <MenuTrigger>{"Действия"}</MenuTrigger>
      <MenuContent>
        <MenuGroup>
          <MenuLabel>{"Версия"}</MenuLabel>
          <MenuItem onClick={onSelect}>{"Скопировать"}</MenuItem>
        </MenuGroup>
        <MenuSeparator />
        <MenuItem variant="destructive">{"Удалить"}</MenuItem>
      </MenuContent>
    </Menu>
  );
}

describe("Menu", () => {
  it("открывается по клику и показывает пункты", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Example onSelect={vi.fn()} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Действия" }));

    // Assert
    expect(await screen.findByRole("menuitem", { name: "Скопировать" })).toBeInTheDocument();
    expect(screen.getByRole("separator")).toBeInTheDocument();
  });

  it("выбор пункта вызывает обработчик и закрывает меню", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<Example onSelect={onSelect} />);
    await user.click(screen.getByRole("button", { name: "Действия" }));

    // Act
    await user.click(await screen.findByRole("menuitem", { name: "Скопировать" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(screen.queryByRole("menu")).not.toBeInTheDocument());
  });
});
