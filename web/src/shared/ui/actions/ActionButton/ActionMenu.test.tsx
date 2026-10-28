import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { type Action, ActionMenu } from "~/shared/ui/actions";

const BLOCKED_ITEM = /Снять блок/;
const REASON_TEXT = /Нужна роль AppSec/;
const STEP_UP_HINT = /требуется подтверждение входа/;

describe("ActionMenu", () => {
  it("пустой список не рендерит ничего", () => {
    // Arrange
    const actions: Action[] = [];

    // Act
    const { container } = render(<ActionMenu actions={actions} />);

    // Assert
    expect(container).toBeEmptyDOMElement();
  });

  it("опасные пункты идут последними, после разделителя", async () => {
    // Arrange
    const user = userEvent.setup();
    const actions: Action[] = [
      { label: "Удалить", run: { onSelect: vi.fn() }, destructive: true },
      { label: "Открыть", run: { href: "/p" } },
      { label: "Скопировать", run: { onSelect: vi.fn() } },
    ];
    render(<ActionMenu actions={actions} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Другие действия" }));

    // Assert
    const items = await screen.findAllByRole("menuitem");
    const separator = screen.getByRole("separator");
    expect(items.map((item) => item.textContent)).toEqual(["Открыть", "Скопировать", "Удалить"]);
    expect(separator.compareDocumentPosition(items[1] as HTMLElement)).toBe(
      Node.DOCUMENT_POSITION_PRECEDING,
    );
    expect(separator.compareDocumentPosition(items[2] as HTMLElement)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
  });

  it("без опасных пунктов разделителя нет", async () => {
    // Arrange
    const user = userEvent.setup();
    const actions: Action[] = [{ label: "Открыть", run: { onSelect: vi.fn() } }];
    render(<ActionMenu actions={actions} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Другие действия" }));

    // Assert
    await screen.findByRole("menuitem", { name: "Открыть" });
    expect(screen.queryByRole("separator")).not.toBeInTheDocument();
  });

  it("выбор пункта с onSelect вызывает обработчик", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(<ActionMenu actions={[{ label: "Скопировать", run: { onSelect } }]} />);
    await user.click(screen.getByRole("button", { name: "Другие действия" }));

    // Act
    await user.click(await screen.findByRole("menuitem", { name: "Скопировать" }));

    // Assert
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("пункт с href — ссылка", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<ActionMenu actions={[{ label: "Открыть", run: { href: "/packages/1" } }]} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Другие действия" }));

    // Assert
    expect(await screen.findByRole("menuitem", { name: "Открыть" })).toHaveAttribute(
      "href",
      "/packages/1",
    );
  });

  it("недоступный пункт показывает причину и не вызывает обработчик", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSelect = vi.fn();
    const actions: Action[] = [
      { label: "Снять блок", run: { onSelect }, disabledReason: "Нужна роль AppSec" },
    ];
    render(<ActionMenu actions={actions} />);
    await user.click(screen.getByRole("button", { name: "Другие действия" }));

    // Act
    await user.click(await screen.findByRole("menuitem", { name: BLOCKED_ITEM }));

    // Assert
    expect(onSelect).not.toHaveBeenCalled();
    expect(screen.getByRole("menuitem", { name: REASON_TEXT })).toHaveAttribute(
      "aria-disabled",
      "true",
    );
  });

  it("step-up пункт получает бейдж MFA и пояснение в имени", async () => {
    // Arrange
    const user = userEvent.setup();
    const actions: Action[] = [{ label: "Применить", run: { onSelect: vi.fn() }, stepUp: true }];
    render(<ActionMenu actions={actions} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Другие действия" }));

    // Assert
    expect(await screen.findByRole("menuitem", { name: STEP_UP_HINT })).toBeInTheDocument();
    expect(screen.getByText("MFA")).toBeVisible();
  });
});
