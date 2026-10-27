import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  Badge,
  Callout,
  CalloutDescription,
  CalloutTitle,
  Card,
  CardAction,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
} from "~/shared/ui/primitives";

const ICON_TONES = ["warning", "error"] as const;

describe("Badge", () => {
  it("показывает текст метки", () => {
    // Arrange
    const text = "yanked";

    // Act
    render(<Badge variant="outline">{text}</Badge>);

    // Assert
    expect(screen.getByText(text)).toBeInTheDocument();
  });

  it("render подменяет элемент, оставляя содержимое", () => {
    // Arrange
    const text = "MFA";

    // Act
    render(<Badge render={<a href="/mfa" />}>{text}</Badge>);

    // Assert
    expect(screen.getByRole("link", { name: text })).toHaveAttribute("href", "/mfa");
  });
});

describe("Card", () => {
  it("собирает заголовок, содержимое и подвал", () => {
    // Arrange
    const title = "Состояние";

    // Act
    render(
      <Card>
        <CardHeader>
          <CardTitle>{title}</CardTitle>
          <CardAction>{"Все"}</CardAction>
        </CardHeader>
        <CardContent>{"Работает"}</CardContent>
        <CardFooter>{"Обновлено"}</CardFooter>
      </Card>,
    );

    // Assert
    expect(screen.getByText(title)).toBeInTheDocument();
    expect(screen.getByText("Все")).toBeInTheDocument();
    expect(screen.getByText("Работает")).toBeInTheDocument();
    expect(screen.getByText("Обновлено")).toBeInTheDocument();
  });
});

describe("Callout", () => {
  it.each(ICON_TONES)("у тона %s есть иконка, статус не передаётся одним цветом", (tone) => {
    // Arrange
    const text = "Требует внимания";

    // Act
    render(<Callout tone={tone}>{text}</Callout>);

    // Assert
    expect(
      screen.getByText(text).closest('[data-slot="callout"]')?.querySelector("svg"),
    ).not.toBeNull();
  });

  it("без live у callout нет роли: это обычный блок в потоке", () => {
    // Arrange
    const text = "Данные обновлены";

    // Act
    render(<Callout tone="info">{text}</Callout>);

    // Assert
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
    expect(screen.queryByRole("alert")).not.toBeInTheDocument();
  });

  it('live="polite" даёт роль status', () => {
    // Arrange
    const text = "Данные обновлены";

    // Act
    render(
      <Callout tone="info" live="polite">
        {text}
      </Callout>,
    );

    // Assert
    expect(screen.getByRole("status")).toHaveTextContent(text);
  });

  it('live="alert" даёт роль alert', () => {
    // Arrange
    const title = "Только чтение";

    // Act
    render(
      <Callout tone="error" live="alert">
        <CalloutTitle>{title}</CalloutTitle>
        <CalloutDescription>{"Запись отключена администратором"}</CalloutDescription>
      </Callout>,
    );

    // Assert
    expect(screen.getByRole("alert")).toHaveTextContent(title);
  });
});
