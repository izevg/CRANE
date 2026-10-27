import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from "~/shared/ui/primitives";

describe("Tooltip", () => {
  it("показывается при фокусе с клавиатуры", async () => {
    // Arrange
    const user = userEvent.setup();
    render(
      <TooltipProvider delay={0}>
        <Tooltip>
          <TooltipTrigger>{"Права"}</TooltipTrigger>
          <TooltipContent>{"Нужна роль AppSec"}</TooltipContent>
        </Tooltip>
      </TooltipProvider>,
    );

    // Act
    await user.tab();

    // Assert
    expect(await screen.findByText("Нужна роль AppSec")).toBeInTheDocument();
  });
});
