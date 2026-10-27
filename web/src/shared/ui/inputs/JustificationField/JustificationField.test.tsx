// biome-ignore-all lint/style/noMagicNumbers: числа в тестах — длины тестовых строк и ожидаемые счётчики
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Harness } from "./Harness";
import { justificationLength, justificationStatus } from "./justification";

const MIN_ERROR = /Минимум 20 символов/;

describe("justificationLength", () => {
  it("пробелы по краям не считаются", () => {
    // Arrange
    const value = "   нужен доступ   ";

    // Act
    const length = justificationLength(value);

    // Assert
    expect(length).toBe(12);
  });

  it("эмодзи и суррогатные пары считаются одним символом", () => {
    // Arrange
    const value = "👩‍💻ab😀";

    // Act
    const length = justificationLength(value);

    // Assert
    expect(length).toBe(Array.from(value).length);
    expect(length).toBeLessThan(value.length);
  });
});

describe("justificationStatus", () => {
  it("ровно минимум — достаточно, на один меньше — недобор", () => {
    // Arrange
    const enough = "а".repeat(20);
    const short = "а".repeat(19);

    // Act
    const actual = [justificationStatus(enough), justificationStatus(short)];

    // Assert
    expect(actual).toEqual(["ok", "short"]);
  });

  it("недобор считается после trim: пробелы не дополняют до минимума", () => {
    // Arrange
    const value = `${" ".repeat(10)}${"а".repeat(15)}${" ".repeat(10)}`;

    // Act
    const status = justificationStatus(value);

    // Assert
    expect(status).toBe("short");
  });
});

describe("JustificationField", () => {
  it("счётчик показывает набранное из минимума", () => {
    // Arrange
    // Act
    render(<Harness label="Обоснование" initial="двенадцать!!" />);

    // Assert
    expect(screen.getByRole("textbox", { name: "Обоснование" })).toHaveAccessibleDescription(
      "12 из 20",
    );
  });

  it("после минимума счётчик показывает остаток до предела", () => {
    // Arrange
    // Act
    render(<Harness label="Обоснование" initial={"а".repeat(25)} />);

    // Assert
    expect(screen.getByRole("textbox", { name: "Обоснование" })).toHaveAccessibleDescription(
      "Осталось 1\u00a0975 символов",
    );
  });

  it("ошибка недобора появляется после ухода из поля и связана с ним", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Harness label="Обоснование" initial="коротко" />);
    await user.click(screen.getByRole("textbox", { name: "Обоснование" }));

    // Act
    await user.tab();

    // Assert
    expect(screen.getByRole("textbox", { name: "Обоснование" })).toHaveAccessibleDescription(
      MIN_ERROR,
    );
    expect(screen.getByRole("textbox", { name: "Обоснование" })).toBeInvalid();
  });

  it("до ухода из поля ошибки нет", () => {
    // Arrange
    // Act
    render(<Harness label="Обоснование" initial="коротко" />);

    // Assert
    expect(screen.queryByText(MIN_ERROR)).not.toBeInTheDocument();
  });

  it("showError показывает ошибку сразу", () => {
    // Arrange
    // Act
    render(<Harness label="Обоснование" initial="коротко" showError />);

    // Assert
    expect(screen.getByRole("textbox", { name: "Обоснование" })).toHaveAccessibleDescription(
      MIN_ERROR,
    );
  });

  it("при достаточной длине ошибки нет даже с showError", () => {
    // Arrange
    // Act
    render(<Harness label="Обоснование" initial={"а".repeat(20)} showError />);

    // Assert
    expect(screen.queryByText(MIN_ERROR)).not.toBeInTheDocument();
  });

  it("textarea ограничена максимумом", () => {
    // Arrange
    // Act
    render(<Harness label="Обоснование" />);

    // Assert
    expect(screen.getByRole("textbox", { name: "Обоснование" })).toHaveAttribute(
      "maxlength",
      "2000",
    );
  });
});
