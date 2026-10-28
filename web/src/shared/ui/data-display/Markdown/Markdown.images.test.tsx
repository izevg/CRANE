import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Markdown } from "~/shared/ui/data-display";

const PIXEL_LINK = /pixel/;
const BUILD_LINK = /build/;
const SHIELDS_LINK = /img\.shields\.io/;
const PIXEL_URL = "https://x.example/p.gif";
const BARE_URL_LINK = /x\.example/;

describe("Markdown: картинки", () => {
  it("внешняя картинка не рисуется как img и не даёт preload", async () => {
    // Arrange
    const source = `# Заголовок\n\n![pixel](${PIXEL_URL})`;

    // Act
    render(<Markdown source={source} />);

    // Assert
    await screen.findByRole("heading", { name: "Заголовок" });
    expect(screen.queryByRole("img")).toBeNull();
    expect(document.querySelector("img")).toBeNull();
    expect(document.querySelector("link[rel=preload]")).toBeNull();
  });

  it("внешняя картинка показывается ссылкой с alt в тексте и защитным rel", async () => {
    // Arrange
    const source = `![pixel](${PIXEL_URL})`;

    // Act
    render(<Markdown source={source} />);

    // Assert
    const link = await screen.findByRole("link", { name: PIXEL_LINK });
    expect(link).toHaveAttribute("href", PIXEL_URL);
    expect(link).toHaveAttribute("target", "_blank");
    expect(link).toHaveAttribute("rel", "noopener noreferrer nofollow");
  });

  it("картинка с пустым alt показывается ссылкой с адресом в тексте", async () => {
    // Arrange
    const source = `![](${PIXEL_URL})`;

    // Act
    render(<Markdown source={source} />);

    // Assert
    const link = await screen.findByRole("link", { name: BARE_URL_LINK });
    expect(link).toHaveAttribute("href", PIXEL_URL);
  });

  it("картинка с data: адресом остаётся текстом alt без img и ссылки", async () => {
    // Arrange
    const source = "# Заголовок\n\n![подпись](data:image/gif;base64,R0lGODlhAQABAAAAACw=)";

    // Act
    render(<Markdown source={source} />);

    // Assert
    await screen.findByRole("heading", { name: "Заголовок" });
    expect(screen.getByText("подпись")).toBeInTheDocument();
    expect(screen.queryByRole("link")).toBeNull();
    expect(document.querySelector("img")).toBeNull();
  });

  it("картинка с относительным адресом остаётся текстом alt без img и ссылки", async () => {
    // Arrange
    const source = "# Заголовок\n\n![схема](./diagram.png)";

    // Act
    render(<Markdown source={source} />);

    // Assert
    await screen.findByRole("heading", { name: "Заголовок" });
    expect(screen.getByText("схема")).toBeInTheDocument();
    expect(screen.queryByRole("link")).toBeNull();
    expect(document.querySelector("img")).toBeNull();
  });

  it("бейдж-картинка внутри ссылки даёт одну ссылку без вложенной", async () => {
    // Arrange
    const source = "[![build](https://img.shields.io/x.svg)](https://ci.example)";

    // Act
    render(<Markdown source={source} />);

    // Assert
    const link = await screen.findByRole("link", { name: BUILD_LINK });
    expect(link).toHaveAttribute("href", "https://ci.example");
    expect(screen.getAllByRole("link")).toHaveLength(1);
    expect(document.querySelector("a a")).toBeNull();
    expect(document.querySelector("img")).toBeNull();
  });

  it("бейдж без alt внутри ссылки показывает адрес картинки текстом", async () => {
    // Arrange
    const source = "[![](https://img.shields.io/x.svg)](https://ci.example)";

    // Act
    render(<Markdown source={source} />);

    // Assert
    const link = await screen.findByRole("link", { name: SHIELDS_LINK });
    expect(link).toHaveAttribute("href", "https://ci.example");
    expect(document.querySelector("a a")).toBeNull();
  });
});
