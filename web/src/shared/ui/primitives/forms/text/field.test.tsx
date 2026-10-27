import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import {
  Field,
  FieldDescription,
  FieldError,
  FieldLabel,
  Input,
  Textarea,
} from "~/shared/ui/primitives";

describe("Field", () => {
  it("ошибка связана с полем через aria-describedby", () => {
    // Arrange
    const message = "Имя уже занято";

    // Act
    render(
      <Field invalid name="name">
        <FieldLabel>{"Имя"}</FieldLabel>
        <Input />
        <FieldError match>{message}</FieldError>
      </Field>,
    );

    // Assert
    expect(screen.getByRole("textbox", { name: "Имя" })).toHaveAccessibleDescription(message);
    expect(screen.getByRole("textbox", { name: "Имя" })).toBeInvalid();
  });

  it("подсказка читается как описание поля", () => {
    // Arrange
    const hint = "До 64 символов";

    // Act
    render(
      <Field name="title">
        <FieldLabel>{"Название"}</FieldLabel>
        <Input />
        <FieldDescription>{hint}</FieldDescription>
      </Field>,
    );

    // Assert
    expect(screen.getByRole("textbox", { name: "Название" })).toHaveAccessibleDescription(hint);
  });

  it("Textarea принимает ввод и связана с подписью", async () => {
    // Arrange
    const user = userEvent.setup();
    render(
      <Field name="reason">
        <FieldLabel>{"Причина"}</FieldLabel>
        <Textarea />
      </Field>,
    );

    // Act
    await user.type(screen.getByRole("textbox", { name: "Причина" }), "Ложное срабатывание");

    // Assert
    expect(screen.getByRole("textbox", { name: "Причина" })).toHaveValue("Ложное срабатывание");
  });
});
