// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовые компоненты-помощники не экспортируются
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  Breadcrumb,
  BreadcrumbItem,
  BreadcrumbLink,
  BreadcrumbList,
  BreadcrumbPage,
  BreadcrumbSeparator,
  type LinkProps,
  LinkProvider,
} from "~/shared/ui/primitives";

function RouterLink(props: LinkProps) {
  return <a data-router="true" {...props} />;
}

function Crumbs() {
  return (
    <Breadcrumb>
      <BreadcrumbList>
        <BreadcrumbItem>
          <BreadcrumbLink href="/packages">{"Пакеты"}</BreadcrumbLink>
        </BreadcrumbItem>
        <BreadcrumbSeparator />
        <BreadcrumbItem>
          <BreadcrumbPage>{"left-pad"}</BreadcrumbPage>
        </BreadcrumbItem>
      </BreadcrumbList>
    </Breadcrumb>
  );
}

describe("Breadcrumb", () => {
  it("помечает текущую страницу aria-current и даёт навигации имя", () => {
    // Arrange
    const current = "left-pad";

    // Act
    render(<Crumbs />);

    // Assert
    expect(screen.getByRole("navigation", { name: "Навигационная цепочка" })).toBeInTheDocument();
    expect(screen.getByText(current)).toHaveAttribute("aria-current", "page");
  });

  it("ссылки идут через LinkProvider", () => {
    // Arrange
    const component = RouterLink;

    // Act
    render(
      <LinkProvider component={component}>
        <Crumbs />
      </LinkProvider>,
    );

    // Assert
    expect(screen.getByRole("link", { name: "Пакеты" })).toHaveAttribute("data-router", "true");
  });
});
