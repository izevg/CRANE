import { render, renderHook, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { usePaletteHotkey } from "~/widgets/command-palette";

describe("usePaletteHotkey", () => {
  it("Ctrl+K открывает палитру", async () => {
    // Arrange
    const user = userEvent.setup();
    const onOpen = vi.fn();
    renderHook(() => usePaletteHotkey(onOpen));

    // Act
    await user.keyboard("{Control>}k{/Control}");

    // Assert
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("Meta+K открывает палитру", async () => {
    // Arrange
    const user = userEvent.setup();
    const onOpen = vi.fn();
    renderHook(() => usePaletteHotkey(onOpen));

    // Act
    await user.keyboard("{Meta>}k{/Meta}");

    // Assert
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("K без модификатора ничего не делает", async () => {
    // Arrange
    const user = userEvent.setup();
    const onOpen = vi.fn();
    renderHook(() => usePaletteHotkey(onOpen));

    // Act
    await user.keyboard("k");

    // Assert
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("Ctrl+Shift+K не перехватывается", async () => {
    // Arrange
    const user = userEvent.setup();
    const onOpen = vi.fn();
    renderHook(() => usePaletteHotkey(onOpen));

    // Act
    await user.keyboard("{Control>}{Shift>}k{/Shift}{/Control}");

    // Assert
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("работает и в фокусе поля ввода", async () => {
    // Arrange
    const user = userEvent.setup();
    const onOpen = vi.fn();
    renderHook(() => usePaletteHotkey(onOpen));
    render(<input aria-label="Поле" />);
    await user.click(screen.getByRole("textbox", { name: "Поле" }));

    // Act
    await user.keyboard("{Control>}k{/Control}");

    // Assert
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("после размонтирования перестаёт слушать", async () => {
    // Arrange
    const user = userEvent.setup();
    const onOpen = vi.fn();
    const { unmount } = renderHook(() => usePaletteHotkey(onOpen));
    unmount();

    // Act
    await user.keyboard("{Control>}k{/Control}");

    // Assert
    expect(onOpen).not.toHaveBeenCalled();
  });
});
