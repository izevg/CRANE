// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые данные (время, размеры, счётчики)
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { toast } from "sonner";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LiveRegion } from "~/shared/lib/a11y";
import { CopyButton } from "~/shared/ui/actions";
import { Toaster } from "~/shared/ui/primitives";

const COPIED_MS = 1200;

afterEach(() => {
  act(() => {
    toast.dismiss();
  });
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("CopyButton", () => {
  it("копирует значение, объявляет «Скопировано» и показывает состояние copied", async () => {
    // Arrange
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
    render(
      <>
        <CopyButton value="guid-1" label="Скопировать GUID" />
        <LiveRegion />
      </>,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Скопировать GUID" }));

    // Assert
    expect(writeText).toHaveBeenCalledWith("guid-1");
    expect(screen.getByRole("button", { name: "Скопировать GUID" })).toHaveAttribute(
      "data-state",
      "copied",
    );
    expect(await screen.findByText("Скопировано")).toBeInTheDocument();
  });

  it("возвращает исходное состояние через 1200 мс", async () => {
    // Arrange
    vi.useFakeTimers({ shouldAdvanceTime: true });
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
    render(<CopyButton value="x" label="Скопировать" />);
    await user.click(screen.getByRole("button", { name: "Скопировать" }));

    // Act
    await act(() => vi.advanceTimersByTimeAsync(COPIED_MS + 100));

    // Assert
    expect(screen.getByRole("button", { name: "Скопировать" })).toHaveAttribute(
      "data-state",
      "idle",
    );
  });

  it("при отказе буфера показывает ошибку и не меняет состояние", async () => {
    // Arrange
    const user = userEvent.setup();
    vi.spyOn(navigator.clipboard, "writeText").mockRejectedValue(new Error("denied"));
    render(
      <>
        <CopyButton value="x" label="Скопировать" />
        <Toaster />
      </>,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Скопировать" }));

    // Assert
    expect(
      await screen.findByText("Не удалось скопировать — выделите текст вручную"),
    ).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Скопировать" })).toHaveAttribute(
      "data-state",
      "idle",
    );
  });
});
