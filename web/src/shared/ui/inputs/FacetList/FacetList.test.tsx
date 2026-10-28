// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые счётчики
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { FacetList } from "~/shared/ui/inputs";

const NPM_NAME = "npm 1\u00a0000+"; // группы разрядов — неразрывный пробел

const ITEMS = [
  { value: "npm", label: "npm", count: { count: 1000, capped: true } },
  { value: "maven", label: "Maven", count: { count: 250, capped: false } },
];

describe("FacetList", () => {
  it("кнопка называется «подпись число», усечённый счётчик со знаком плюс", () => {
    // Arrange
    const onToggle = vi.fn();

    // Act
    render(<FacetList title="По форматам" items={ITEMS} selected={[]} onToggle={onToggle} />);

    // Assert
    expect(screen.getByRole("button", { name: NPM_NAME })).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Maven 250" })).toBeInTheDocument();
  });

  it("заголовок второго уровня по умолчанию и третьего по запросу", () => {
    // Arrange
    const onToggle = vi.fn();

    // Act
    render(
      <FacetList
        title="По форматам"
        headingLevel={3}
        items={ITEMS}
        selected={[]}
        onToggle={onToggle}
      />,
    );

    // Assert
    expect(screen.getByRole("heading", { level: 3, name: "По форматам" })).toBeInTheDocument();
  });

  it("выбранные значения помечены aria-pressed", () => {
    // Arrange
    const onToggle = vi.fn();

    // Act
    render(
      <FacetList title="По форматам" items={ITEMS} selected={["maven"]} onToggle={onToggle} />,
    );

    // Assert
    expect(screen.getByRole("button", { name: "Maven 250" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(screen.getByRole("button", { name: NPM_NAME })).toHaveAttribute("aria-pressed", "false");
  });

  it("у нажатой строки есть галочка, у ненажатой нет", () => {
    // Arrange
    const onToggle = vi.fn();

    // Act
    render(
      <FacetList title="По форматам" items={ITEMS} selected={["maven"]} onToggle={onToggle} />,
    );

    // Assert
    const pressed = screen.getByRole("button", { name: "Maven 250" });
    const idle = screen.getByRole("button", { name: NPM_NAME });
    expect(pressed.querySelector('[data-slot="facet-check"]')).not.toBeNull();
    expect(idle.querySelector('[data-slot="facet-check"]')).toBeNull();
  });

  it("клик по строке вызывает onToggle со значением", async () => {
    // Arrange
    const user = userEvent.setup();
    const onToggle = vi.fn();
    render(<FacetList title="По форматам" items={ITEMS} selected={[]} onToggle={onToggle} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Maven 250" }));

    // Assert
    expect(onToggle).toHaveBeenCalledWith("maven");
  });

  it("пустой список ничего не рисует", () => {
    // Arrange
    const onToggle = vi.fn();

    // Act
    const { container } = render(
      <FacetList title="По форматам" items={[]} selected={[]} onToggle={onToggle} />,
    );

    // Assert
    expect(container).toBeEmptyDOMElement();
  });
});
