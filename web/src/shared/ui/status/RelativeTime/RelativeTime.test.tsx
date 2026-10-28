// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые данные (время, размеры, счётчики)
import { act, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { RelativeTime } from "~/shared/ui/status";

const NOW = new Date(2026, 9, 27, 12, 0, 0);
const MINUTE_MS = 60_000;

beforeEach(() => {
  vi.useFakeTimers();
  vi.setSystemTime(NOW);
});

afterEach(() => {
  vi.useRealTimers();
});

describe("RelativeTime", () => {
  it("показывает относительное время", () => {
    // Arrange
    const ts = NOW.getTime() - 5 * MINUTE_MS;

    // Act
    render(<RelativeTime ts={ts} />);

    // Assert
    expect(screen.getByText("5 мин. назад")).toBeInTheDocument();
  });

  it("отдаёт машиночитаемое время и полную дату в подсказке", () => {
    // Arrange
    const ts = new Date(2026, 9, 27, 11, 30, 0);

    // Act
    render(<RelativeTime ts={ts} />);

    // Assert
    const time = screen.getByText("30 мин. назад");
    expect(time.tagName).toBe("TIME");
    expect(time).toHaveAttribute("datetime", ts.toISOString());
    expect(time).toHaveAttribute("title", expect.stringContaining("27 октября 2026"));
  });

  it("обновляется раз в минуту", async () => {
    // Arrange
    const ts = NOW.getTime() - 5 * MINUTE_MS;
    render(<RelativeTime ts={ts} />);

    // Act
    await act(() => vi.advanceTimersByTimeAsync(MINUTE_MS));

    // Assert
    expect(screen.getByText("6 мин. назад")).toBeInTheDocument();
  });

  it("не является живым регионом", () => {
    // Arrange
    const ts = NOW.getTime() - MINUTE_MS;

    // Act
    render(<RelativeTime ts={ts} />);

    // Assert
    expect(screen.getByText("1 мин. назад")).not.toHaveAttribute("aria-live");
  });

  it("некорректная дата показывается прочерком без datetime", () => {
    // Arrange
    const ts = "не дата";

    // Act
    render(<RelativeTime ts={ts} />);

    // Assert
    expect(screen.getByText("—")).not.toHaveAttribute("datetime");
  });
});
