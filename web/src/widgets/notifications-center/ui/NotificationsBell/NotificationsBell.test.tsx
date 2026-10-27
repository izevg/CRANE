// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые счётчики
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { NotificationsBell } from "~/widgets/notifications-center";

describe("NotificationsBell", () => {
  it("число непрочитанных входит в доступное имя", () => {
    // Arrange
    const unread = 3;

    // Act
    render(<NotificationsBell unread={unread} />);

    // Assert
    expect(
      screen.getByRole("button", { name: "Уведомления, 3 непрочитанных" }),
    ).toBeInTheDocument();
    expect(screen.getByText("3")).toBeInTheDocument();
  });

  it("одно уведомление склоняется в единственном числе", () => {
    // Arrange
    const unread = 1;

    // Act
    render(<NotificationsBell unread={unread} />);

    // Assert
    expect(
      screen.getByRole("button", { name: "Уведомления, 1 непрочитанное" }),
    ).toBeInTheDocument();
  });

  it("21 уведомление склоняется как единственное число", () => {
    // Arrange
    const unread = 21;

    // Act
    render(<NotificationsBell unread={unread} />);

    // Assert
    expect(
      screen.getByRole("button", { name: "Уведомления, 21 непрочитанное" }),
    ).toBeInTheDocument();
  });

  it("пять уведомлений склоняются во множественном числе", () => {
    // Arrange
    const unread = 5;

    // Act
    render(<NotificationsBell unread={unread} />);

    // Assert
    expect(
      screen.getByRole("button", { name: "Уведомления, 5 непрочитанных" }),
    ).toBeInTheDocument();
  });

  it("больше девяти: на значке «9+», а точное число — в имени", () => {
    // Arrange
    const unread = 12;

    // Act
    render(<NotificationsBell unread={unread} />);

    // Assert
    expect(
      screen.getByRole("button", { name: "Уведомления, 12 непрочитанных" }),
    ).toBeInTheDocument();
    expect(screen.getByText("9+")).toBeInTheDocument();
  });

  it("ровно девять: значок показывает число без плюса", () => {
    // Arrange
    const unread = 9;

    // Act
    render(<NotificationsBell unread={unread} />);

    // Assert
    expect(screen.getByText("9")).toBeInTheDocument();
    expect(screen.queryByText("9+")).not.toBeInTheDocument();
  });

  it("без непрочитанных: имя без числа и значка нет", () => {
    // Arrange
    const unread = 0;

    // Act
    render(<NotificationsBell unread={unread} />);

    // Assert
    const bell = screen.getByRole("button", { name: "Уведомления" });
    expect(bell).toHaveTextContent("");
  });
});
