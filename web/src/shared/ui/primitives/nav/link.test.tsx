// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовые компоненты-помощники не экспортируются
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Link, type LinkProps, LinkProvider } from "~/shared/ui/primitives";

function RouterLink({ href, ...props }: LinkProps) {
  return <a href={`/app${href}`} data-router="true" {...props} />;
}

describe("Link", () => {
  it("без провайдера рендерит обычный <a href>", () => {
    // Arrange
    const href = "/packages";

    // Act
    render(<Link href={href}>{"Пакеты"}</Link>);

    // Assert
    expect(screen.getByRole("link", { name: "Пакеты" })).toHaveAttribute("href", href);
  });

  it("с провайдером рендерит переданный компонент ссылки", () => {
    // Arrange
    const href = "/packages";

    // Act
    render(
      <LinkProvider component={RouterLink}>
        <Link href={href}>{"Пакеты"}</Link>
      </LinkProvider>,
    );

    // Assert
    expect(screen.getByRole("link", { name: "Пакеты" })).toHaveAttribute("data-router", "true");
    expect(screen.getByRole("link", { name: "Пакеты" })).toHaveAttribute("href", "/app/packages");
  });
});
