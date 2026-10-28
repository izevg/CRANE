// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовые компоненты-помощники не экспортируются
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "~/shared/ui/primitives";

function PackageTabs({ variant }: { variant: "default" | "line" }) {
  return (
    <Tabs defaultValue="readme">
      <TabsList variant={variant} aria-label="Разделы">
        <TabsTrigger value="readme">{"README"}</TabsTrigger>
        <TabsTrigger value="versions">{"Версии"}</TabsTrigger>
      </TabsList>
      <TabsContent value="readme">{"Описание пакета"}</TabsContent>
      <TabsContent value="versions">{"Список версий"}</TabsContent>
    </Tabs>
  );
}

const VARIANTS = ["default", "line"] as const;

describe("Tabs", () => {
  it.each(VARIANTS)("вариант %s: стрелка вправо переключает вкладку и панель", async (variant) => {
    // Arrange
    const user = userEvent.setup();
    render(<PackageTabs variant={variant} />);
    await user.click(screen.getByRole("tab", { name: "README" }));

    // Act
    await user.keyboard("{ArrowRight}");

    // Assert
    expect(screen.getByRole("tab", { name: "Версии" })).toHaveAttribute("aria-selected", "true");
    expect(screen.getByRole("tabpanel")).toHaveTextContent("Список версий");
  });

  it("активной по умолчанию остаётся первая вкладка", () => {
    // Arrange
    const first = "README";

    // Act
    render(<PackageTabs variant="default" />);

    // Assert
    expect(screen.getByRole("tab", { name: first })).toHaveAttribute("aria-selected", "true");
  });
});
