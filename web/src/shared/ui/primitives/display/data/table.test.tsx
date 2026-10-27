import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import {
  Skeleton,
  Table,
  TableBody,
  TableCaption,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "~/shared/ui/primitives";

describe("Table", () => {
  it("строит таблицу с заголовками столбцов и подписью", () => {
    // Arrange
    const caption = "Версии пакета";

    // Act
    render(
      <Table>
        <TableCaption>{caption}</TableCaption>
        <TableHeader>
          <TableRow>
            <TableHead>{"Версия"}</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          <TableRow>
            <TableCell>{"1.4.2"}</TableCell>
          </TableRow>
        </TableBody>
      </Table>,
    );

    // Assert
    expect(screen.getByRole("table", { name: caption })).toBeInTheDocument();
    expect(screen.getByRole("columnheader", { name: "Версия" })).toBeInTheDocument();
    expect(screen.getByRole("cell", { name: "1.4.2" })).toBeInTheDocument();
  });
});

describe("Skeleton", () => {
  it("скрыт от скринридера", () => {
    // Arrange
    const testId = "skeleton";

    // Act
    render(<Skeleton data-testid={testId} />);

    // Assert
    expect(screen.getByTestId(testId)).toHaveAttribute("aria-hidden", "true");
  });
});
