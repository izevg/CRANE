import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { type Action, ActionButton } from "~/shared/ui/actions";

const STEP_UP_HINT = /требуется подтверждение входа/;

const REASON = "Нужна роль AppSec";

describe("ActionButton", () => {
  it("действие с href рендерится ссылкой", () => {
    // Arrange
    const action: Action = { label: "Открыть", run: { href: "/packages/left-pad" } };

    // Act
    render(<ActionButton action={action} variant="secondary" />);

    // Assert
    expect(screen.getByRole("link", { name: "Открыть" })).toHaveAttribute(
      "href",
      "/packages/left-pad",
    );
  });

  it("действие с onSelect рендерится кнопкой и вызывает обработчик", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <ActionButton action={{ label: "Заблокировать", run: { onSelect } }} variant="primary" />,
    );

    // Act
    await user.click(screen.getByRole("button", { name: "Заблокировать" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("действие с причиной недоступности: aria-disabled и причина в описании", () => {
    // Arrange
    const action: Action = {
      label: "Снять блок",
      run: { onSelect: vi.fn() },
      disabledReason: REASON,
    };

    // Act
    render(<ActionButton action={action} variant="primary" />);

    // Assert
    const button = screen.getByRole("button", { name: "Снять блок" });
    expect(button).toHaveAttribute("aria-disabled", "true");
    expect(button).toHaveAccessibleDescription(REASON);
  });

  it("клик по недоступному действию не вызывает обработчик", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const action: Action = { label: "Снять блок", run: { onSelect }, disabledReason: REASON };
    render(<ActionButton action={action} variant="primary" />);

    // Act
    await user.click(screen.getByRole("button", { name: "Снять блок" }));

    // Assert
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("недоступное действие остаётся фокусируемым с клавиатуры", async () => {
    // Arrange
    const user = userEvent.setup();
    const action: Action = {
      label: "Снять блок",
      run: { onSelect: vi.fn() },
      disabledReason: REASON,
    };
    render(<ActionButton action={action} variant="primary" />);

    // Act
    await user.tab();

    // Assert
    expect(screen.getByRole("button", { name: "Снять блок" })).toHaveFocus();
  });

  it("недоступное действие с href не становится ссылкой", () => {
    // Arrange
    const action: Action = { label: "Открыть", run: { href: "/x" }, disabledReason: REASON };

    // Act
    render(<ActionButton action={action} variant="secondary" />);

    // Assert
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Открыть" })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
  });

  it("причина недоступности показывается подсказкой при фокусе", async () => {
    // Arrange
    const user = userEvent.setup();
    const action: Action = {
      label: "Снять блок",
      run: { onSelect: vi.fn() },
      disabledReason: REASON,
    };
    render(<ActionButton action={action} variant="primary" />);

    // Act
    await user.tab();

    // Assert
    expect(await screen.findAllByText(REASON)).toHaveLength(2);
  });

  it("step-up: бейдж MFA виден, а в имени есть пояснение", () => {
    // Arrange
    const action: Action = {
      label: "Применить retention",
      run: { onSelect: vi.fn() },
      stepUp: true,
    };

    // Act
    render(<ActionButton action={action} variant="primary" />);

    // Assert
    expect(screen.getByText("MFA")).toBeVisible();
    expect(screen.getByRole("button")).toHaveAccessibleName(STEP_UP_HINT);
  });

  it("обычное действие не получает бейдж MFA", () => {
    // Arrange
    const action: Action = { label: "Прогрев", run: { onSelect: vi.fn() } };

    // Act
    render(<ActionButton action={action} variant="ghost" />);

    // Assert
    expect(screen.queryByText("MFA")).not.toBeInTheDocument();
  });
});
