import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Field, FieldLabel, PasswordInput } from "~/shared/ui/primitives";

const LABEL = "Токен доступа";
const TOGGLE = "Показать пароль";
const TOKEN = "crb_9f2c71aa41d0";

function field(props: Partial<Parameters<typeof PasswordInput>[0]> = {}) {
  return (
    <Field>
      <FieldLabel>{LABEL}</FieldLabel>
      <PasswordInput {...props} />
    </Field>
  );
}

describe("PasswordInput", () => {
  it("по умолчанию скрывает ввод и подписан полем Field", () => {
    // Arrange
    const props = {};

    // Act
    const { container } = render(field(props));

    // Assert
    expect(container.querySelector("input")).toHaveAttribute("type", "password");
    expect(screen.getByLabelText(LABEL)).toBeInTheDocument();
  });

  it("кнопка показывает значение и сообщает aria-pressed", async () => {
    // Arrange
    const user = userEvent.setup();
    const { container } = render(field());

    // Act
    await user.click(screen.getByRole("button", { name: TOGGLE }));

    // Assert
    expect(container.querySelector("input")).toHaveAttribute("type", "text");
    expect(screen.getByRole("button", { name: TOGGLE })).toHaveAttribute("aria-pressed", "true");
  });

  it("повторное нажатие снова скрывает значение", async () => {
    // Arrange
    const user = userEvent.setup();
    const { container } = render(field());
    await user.click(screen.getByRole("button", { name: TOGGLE }));

    // Act
    await user.click(screen.getByRole("button", { name: TOGGLE }));

    // Assert
    expect(container.querySelector("input")).toHaveAttribute("type", "password");
    expect(screen.getByRole("button", { name: TOGGLE })).toHaveAttribute("aria-pressed", "false");
  });

  it("имя кнопки задаётся toggleLabel", () => {
    // Arrange
    const props = { toggleLabel: "Показать токен" };

    // Act
    render(field(props));

    // Assert
    expect(screen.getByRole("button", { name: "Показать токен" })).toBeInTheDocument();
  });

  it("кнопка управляет полем через aria-controls", () => {
    // Arrange
    const props = {};

    // Act
    const { container } = render(field(props));

    // Assert
    const input = container.querySelector("input");
    expect(screen.getByRole("button", { name: TOGGLE })).toHaveAttribute(
      "aria-controls",
      input?.id,
    );
  });

  it("выключает подсказки клавиатуры и автоисправление по умолчанию", () => {
    // Arrange
    const props = {};

    // Act
    const { container } = render(field(props));

    // Assert
    const input = container.querySelector("input");
    expect(input).toHaveAttribute("spellcheck", "false");
    expect(input).toHaveAttribute("autocapitalize", "off");
    expect(input).toHaveAttribute("autocorrect", "off");
  });

  it("пропускает autocomplete вызывающего", () => {
    // Arrange
    const props = { autoComplete: "current-password" };

    // Act
    render(field(props));

    // Assert
    expect(screen.getByLabelText(LABEL)).toHaveAttribute("autocomplete", "current-password");
  });

  it("не ограничивает длину", () => {
    // Arrange
    const props = {};

    // Act
    render(field(props));

    // Assert
    expect(screen.getByLabelText(LABEL)).not.toHaveAttribute("maxlength");
  });

  it("вставка токена сохраняется целиком", async () => {
    // Arrange
    const user = userEvent.setup();
    render(field());
    await user.click(screen.getByLabelText(LABEL));

    // Act
    await user.paste(TOKEN);

    // Assert
    expect(screen.getByLabelText(LABEL)).toHaveValue(TOKEN);
  });

  it("пробелы по краям вставки не режутся: trim делает вызывающий", async () => {
    // Arrange
    const user = userEvent.setup();
    render(field());
    await user.click(screen.getByLabelText(LABEL));

    // Act
    await user.paste(`  ${TOKEN}  `);

    // Assert
    expect(screen.getByLabelText(LABEL)).toHaveValue(`  ${TOKEN}  `);
  });

  it("перевод строки внутри вставки браузер убирает сам, компонент остальное не трогает", async () => {
    // Arrange
    const user = userEvent.setup();
    render(field());
    await user.click(screen.getByLabelText(LABEL));

    // Act
    await user.paste("crb_9f2c\n71aa41d0");

    // Assert
    expect(screen.getByLabelText(LABEL)).toHaveValue("crb_9f2c71aa41d0");
  });
});
