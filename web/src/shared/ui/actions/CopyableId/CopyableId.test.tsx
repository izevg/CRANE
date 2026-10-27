import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LiveRegion } from "~/shared/lib/a11y";
import { CopyableId } from "~/shared/ui/actions";

const COPY_REQUEST_ID = /Скопировать request_id/;

const FULL = "a88d1f3c-77be-4a6e-9d21-5be0c1d46973";

afterEach(() => {
  vi.restoreAllMocks();
});

describe("CopyableId", () => {
  it("показывает значение, усечённое в середине", () => {
    // Arrange
    const value = FULL;

    // Act
    render(<CopyableId value={value} />);

    // Assert
    expect(screen.getByText("a88d…6973")).toBeInTheDocument();
  });

  it("полное значение есть в доступном имени кнопки", () => {
    // Arrange
    const value = FULL;

    // Act
    render(<CopyableId value={value} />);

    // Assert
    expect(screen.getByRole("button")).toHaveAccessibleName(`Скопировать ID: ${FULL}`);
  });

  it("полное значение есть в подсказке", () => {
    // Arrange
    const value = FULL;

    // Act
    render(<CopyableId value={value} />);

    // Assert
    expect(screen.getByTitle(FULL)).toBeInTheDocument();
  });

  it("копирует полное значение, а не усечённое, и объявляет «Скопировано»", async () => {
    // Arrange
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText").mockResolvedValue();
    render(
      <>
        <CopyableId value={FULL} label="Скопировать request_id" />
        <LiveRegion />
      </>,
    );

    // Act
    await user.click(screen.getByRole("button", { name: COPY_REQUEST_ID }));

    // Assert
    expect(writeText).toHaveBeenCalledWith(FULL);
    expect(await screen.findByText("Скопировано")).toBeInTheDocument();
  });

  it("с full показывает значение целиком", () => {
    // Arrange
    const value = FULL;

    // Act
    render(<CopyableId value={value} full />);

    // Assert
    expect(screen.getByText(FULL)).toBeVisible();
  });

  it("короткое значение не сокращается", () => {
    // Arrange
    const value = "abc123";

    // Act
    render(<CopyableId value={value} />);

    // Assert
    expect(screen.getByText("abc123")).toBeInTheDocument();
  });
});
