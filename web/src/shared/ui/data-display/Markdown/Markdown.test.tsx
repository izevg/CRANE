import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Markdown } from "~/shared/ui/data-display";

const DOCS_LINK = /документацию/;
const SITE_LINK = /сайт/;
const CLICK_LINK = /нажми/;

describe("Markdown", () => {
  it("рисует заголовок и ссылку после загрузки ленивого модуля", async () => {
    // Arrange
    const source = "# Установка\n\nСмотрите [документацию](https://example.com/docs).";

    // Act
    render(<Markdown source={source} />);

    // Assert
    expect(await screen.findByRole("heading", { name: "Установка" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: DOCS_LINK })).toHaveAttribute(
      "href",
      "https://example.com/docs",
    );
  });

  it("внешняя ссылка открывается в новой вкладке без opener", async () => {
    // Arrange
    const source = "[сайт](https://example.com)";

    // Act
    render(<Markdown source={source} />);

    // Assert
    const link = await screen.findByRole("link", { name: SITE_LINK });
    expect(link).toHaveAttribute("target", "_blank");
    expect(link).toHaveAttribute("rel", "noopener noreferrer");
  });

  it("ссылка javascript: теряет адрес и не становится ссылкой", async () => {
    // Arrange
    const source = "# Заголовок\n\n[нажми](javascript:alert(1))";

    // Act
    render(<Markdown source={source} />);

    // Assert
    await screen.findByRole("heading", { name: "Заголовок" });
    expect(screen.queryByRole("link", { name: CLICK_LINK })).not.toBeInTheDocument();
    expect(screen.getByText(CLICK_LINK)).toBeInTheDocument();
  });

  it("сырой <script> не попадает в документ", async () => {
    // Arrange
    const source = "# Заголовок\n\n<script>window.hacked = true</script>";

    // Act
    render(<Markdown source={source} />);

    // Assert
    await screen.findByRole("heading", { name: "Заголовок" });
    expect(document.querySelector("script")).toBeNull();
  });

  it("сырой HTML с onerror не создаёт элемент и обработчик", async () => {
    // Arrange
    const source = "# Заголовок\n\n<img src=x onerror=alert(1)>";

    // Act
    render(<Markdown source={source} />);

    // Assert
    await screen.findByRole("heading", { name: "Заголовок" });
    expect(document.querySelector("[onerror]")).toBeNull();
    expect(screen.queryByRole("img")).not.toBeInTheDocument();
  });
});
