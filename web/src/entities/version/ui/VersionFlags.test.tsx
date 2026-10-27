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

  it("сообщение об устаревании показывается подсказкой при фокусе", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<VersionFlags flags={{ ...NONE, deprecated_message: "Используйте 2.x" }} />);

    // Act
    await user.tab();

    // Assert
    expect((await screen.findAllByText("Используйте 2.x")).length).toBeGreaterThan(0);
  });

  it("сообщение об устаревании входит в доступное описание флага", () => {
    // Arrange
    // Act
    render(<VersionFlags flags={{ ...NONE, deprecated_message: "Используйте 2.x" }} />);

    // Assert
    expect(screen.getByText("deprecated")).toHaveAccessibleDescription("Используйте 2.x");
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
