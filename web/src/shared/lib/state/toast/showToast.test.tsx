import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { toast } from "sonner";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { LiveRegion } from "~/shared/lib/a11y";
import { showToast } from "~/shared/lib/state";
import { Toaster } from "~/shared/ui/primitives";

const LONG_WAIT_MS = 60_000;

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true });
});

afterEach(() => {
  act(() => {
    toast.dismiss();
  });
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("showToast", () => {
  it("успех исчезает сам", async () => {
    // Arrange
    render(<Toaster />);
    act(() => showToast.success("Токен создан"));
    await screen.findByText("Токен создан");

    // Act
    await act(() => vi.advanceTimersByTimeAsync(LONG_WAIT_MS));

    // Assert
    expect(screen.queryByText("Токен создан")).not.toBeInTheDocument();
  });

  it("ошибка с request_id не исчезает по таймеру и показывает идентификатор", async () => {
    // Arrange
    render(<Toaster />);
    act(() => showToast.error({ title: "Не удалось отозвать токен", requestId: "req-7f3a" }));

    // Act
    await act(() => vi.advanceTimersByTimeAsync(LONG_WAIT_MS));

    // Assert
    expect(screen.getByText("Не удалось отозвать токен")).toBeInTheDocument();
    expect(screen.getByText("request_id: req-7f3a")).toBeInTheDocument();
  });

  it("кнопка копирует request_id и оставляет тост на экране", async () => {
    // Arrange
    const user = userEvent.setup({ advanceTimers: vi.advanceTimersByTime });
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
    render(
      <>
        <Toaster />
        <LiveRegion />
      </>,
    );
    act(() => showToast.error({ title: "Сбой", requestId: "req-1" }));

    // Act
    await user.click(await screen.findByRole("button", { name: "Копировать request_id" }));

    // Assert
    expect(writeText).toHaveBeenCalledWith("req-1");
    expect(screen.getByText("Сбой")).toBeInTheDocument();
  });

  it("ошибка без request_id не содержит кнопки копирования", async () => {
    // Arrange
    render(<Toaster />);

    // Act
    act(() => showToast.error({ title: "Сеть недоступна" }));

    // Assert
    expect(await screen.findByText("Сеть недоступна")).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: "Копировать request_id" })).not.toBeInTheDocument();
  });

  it("info показывает сообщение", async () => {
    // Arrange
    render(<Toaster />);

    // Act
    act(() => showToast.info("Поставлено в очередь: 41"));

    // Assert
    expect(await screen.findByText("Поставлено в очередь: 41")).toBeInTheDocument();
  });
});
