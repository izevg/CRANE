// biome-ignore-all lint/style/noMagicNumbers: числа в тестах — длины тестовых строк и ожидаемые счётчики
// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовая обёртка не экспортируется
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useState } from "react";
import { describe, expect, it } from "vitest";
import { JustificationField } from "./JustificationField";
import { isJustified, justificationLength, justificationStatus } from "./justification";

const MIN_ERROR = /Минимум 20 символов/;
const MAX_ERROR = /Максимум 2\s000 символов/;

/** Управляемая обёртка: поле хранит значение само, как это делает форма. */
function Harness({ initial = "", showError = false }: { initial?: string; showError?: boolean }) {
  const [value, setValue] = useState(initial);
  return (
    <JustificationField
      label="Обоснование"
      value={value}
      onChange={setValue}
      showError={showError}
    />
  );
}

const field = () => screen.getByRole("textbox", { name: "Обоснование" });

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

describe("justificationStatus и isJustified", () => {
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

  it("ровно максимум в code points — достаточно, даже если UTF-16 длина больше", () => {
    // Arrange
    const value = `${"а".repeat(1999)}😀`;

    // Act
    const status = justificationStatus(value);

    // Assert
    expect(status).toBe("ok");
    expect(value.length).toBeGreaterThan(2000);
  });

  it("на один символ больше максимума — превышение", () => {
    // Arrange
    const value = "а".repeat(2001);

    // Act
    const actual = [justificationStatus(value), isJustified(value)];

    // Assert
    expect(actual).toEqual(["long", false]);
  });

  it("isJustified истинно только для допустимой длины", () => {
    // Arrange
    const values = ["", "а".repeat(20), "а".repeat(2000), "а".repeat(2001)];

    // Act
    const actual = values.map(isJustified);

    // Assert
    expect(actual).toEqual([false, true, true, false]);
  });
});

describe("JustificationField", () => {
  it("поле обязательное: required и aria-required", () => {
    // Arrange
    // Act
    render(<Harness />);

    // Assert
    expect(field()).toBeRequired();
    expect(field()).toHaveAttribute("aria-required", "true");
  });

  it("счётчик показывает набранное из минимума", () => {
    // Arrange
    // Act
    render(<Harness initial="двенадцать!!" />);

    // Assert
    expect(field()).toHaveAccessibleDescription("12 из 20");
  });

  it("после минимума счётчик показывает остаток до предела", () => {
    // Arrange
    // Act
    render(<Harness initial={"а".repeat(25)} />);

    // Assert
    expect(field()).toHaveAccessibleDescription("Осталось 1 975 символов");
  });

  it("остаток склоняется по правилам языка: 21 — «Остался 21 символ»", () => {
    // Arrange
    // Act
    render(<Harness initial={"а".repeat(1979)} />);

    // Assert
    expect(field()).toHaveAccessibleDescription("Остался 21 символ");
  });

  it("native maxLength нет: ввод сверх предела не обрезается молча", () => {
    // Arrange
    // Act
    render(<Harness initial={"а".repeat(2001)} />);

    // Assert
    expect(field()).not.toHaveAttribute("maxlength");
  });

  it("превышение предела показывает ошибку «Максимум 2000 символов»", () => {
    // Arrange
    // Act
    render(<Harness initial={"а".repeat(2001)} />);

    // Assert
    expect(field()).toHaveAccessibleDescription(MAX_ERROR);
    expect(field()).toBeInvalid();
  });

  it("ошибка недобора появляется после ухода из поля и связана с ним", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<Harness initial="коротко" />);
    await user.click(field());

    // Act
    await user.tab();

    // Assert
    expect(field()).toHaveAccessibleDescription(MIN_ERROR);
    expect(field()).toBeInvalid();
  });

  it("до ухода из поля ошибки нет", () => {
    // Arrange
    // Act
    render(<Harness initial="коротко" />);

    // Assert
    expect(screen.queryByText(MIN_ERROR)).not.toBeInTheDocument();
  });

  it("showError показывает ошибку сразу", () => {
    // Arrange
    // Act
    render(<Harness initial="коротко" showError />);

    // Assert
    expect(field()).toHaveAccessibleDescription(MIN_ERROR);
  });

  it("при достаточной длине ошибки нет даже с showError", () => {
    // Arrange
    // Act
    render(<Harness initial={"а".repeat(20)} showError />);

    // Assert
    expect(screen.queryByText(MIN_ERROR)).not.toBeInTheDocument();
  });
});
