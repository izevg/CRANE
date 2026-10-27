import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { VersionFlags as Flags } from "~/shared/api";
import { VersionFlags } from "../index";

const NONE: Flags = {
  yanked: false,
  unlisted: false,
  deprecated_message: null,
  dist_tags: [],
  mutable: false,
};

describe("VersionFlags", () => {
  it("без флагов не рисует ничего, даже пустого контейнера", () => {
    // Arrange
    // Act
    const { container } = render(<VersionFlags flags={NONE} />);

    // Assert
    expect(container).toBeEmptyDOMElement();
  });

  it("каждый флаг и dist-tag — отдельный бейдж", () => {
    // Arrange
    const flags: Flags = { ...NONE, yanked: true, mutable: true, dist_tags: ["latest", "next"] };

    // Act
    render(<VersionFlags flags={flags} />);

    // Assert
    expect(
      ["yanked", "mutable", "latest", "next"].map((name) => screen.getByText(name).textContent),
    ).toEqual(["yanked", "mutable", "latest", "next"]);
  });

  it("у deprecated есть кнопка с именем, а бейдж остаётся текстом", () => {
    // Arrange
    // Act
    render(<VersionFlags flags={{ ...NONE, deprecated_message: "Используйте 2.x" }} />);

    // Assert
    expect(screen.getByRole("button", { name: "Подробнее: deprecated" })).toBeVisible();
    expect(screen.getByText("deprecated")).not.toHaveAttribute("tabindex");
  });

  it("нажатие на кнопку открывает сообщение автора об устаревании", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<VersionFlags flags={{ ...NONE, deprecated_message: "Используйте 2.x" }} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Подробнее: deprecated" }));

    // Assert
    expect(await screen.findByText("Используйте 2.x")).toBeVisible();
  });

  it("у флагов без пояснения кнопки нет", () => {
    // Arrange
    // Act
    render(<VersionFlags flags={{ ...NONE, yanked: true }} />);

    // Assert
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  it("флаги идут в порядке: статусные, затем dist-tags", () => {
    // Arrange
    const flags: Flags = { ...NONE, unlisted: true, yanked: true, dist_tags: ["latest"] };

    // Act
    render(<VersionFlags flags={flags} />);

    // Assert
    const container = screen.getByText("latest").parentElement;
    expect(container?.textContent).toBe("yankedunlistedlatest");
  });
});
