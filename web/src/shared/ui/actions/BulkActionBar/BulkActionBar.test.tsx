import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LiveRegion } from "~/shared/lib/a11y";
import { type Action, BulkActionBar } from "~/shared/ui/actions";

const REGION = "Массовые действия";
const CLEAR = "Снять выделение";
const ACTIONS: Action[] = [
  { label: "Принять VEX", run: { onSelect: vi.fn() } },
  { label: "Создать задачу", run: { href: "/issues/new" } },
];

afterEach(() => {
  document.documentElement.style.removeProperty("scroll-padding-bottom");
  vi.restoreAllMocks();
});

describe("BulkActionBar", () => {
  it("при count 0 ничего не рендерится", () => {
    // Arrange
    const count = 0;

    // Act
    const { container } = render(
      <BulkActionBar count={count} actions={ACTIONS} onClear={vi.fn()} />,
    );

    // Assert
    expect(container).toBeEmptyDOMElement();
  });

  it("показывает «Выбрано 3» в именованной группе", () => {
    // Arrange
    const count = 3;

    // Act
    render(<BulkActionBar count={count} actions={ACTIONS} onClear={vi.fn()} />);

    // Assert
    const toolbar = screen.getByRole("group", { name: REGION });
    expect(toolbar).toHaveTextContent("Выбрано 3");
  });

  it("кнопки есть только для переданных действий", () => {
    // Arrange
    const actions: Action[] = [{ label: "Принять VEX", run: { onSelect: vi.fn() } }];

    // Act
    render(<BulkActionBar count={2} actions={actions} onClear={vi.fn()} />);

    // Assert
    expect(screen.getByRole("button", { name: "Принять VEX" })).toBeInTheDocument();
    expect(screen.queryByRole("link")).toBeNull();
    expect(screen.queryByRole("button", { name: "Экспорт" })).toBeNull();
  });

  it("действие-ссылка рендерится ссылкой", () => {
    // Arrange
    const count = 2;

    // Act
    render(<BulkActionBar count={count} actions={ACTIONS} onClear={vi.fn()} />);

    // Assert
    expect(screen.getByRole("link", { name: "Создать задачу" })).toHaveAttribute(
      "href",
      "/issues/new",
    );
  });

  it("недоступное действие остаётся с причиной", () => {
    // Arrange
    const reason = "VEX выдаётся на один бюллетень";
    const actions: Action[] = [
      { label: "Принять VEX", run: { onSelect: vi.fn() }, disabledReason: reason },
    ];

    // Act
    render(<BulkActionBar count={2} actions={actions} onClear={vi.fn()} />);

    // Assert
    expect(screen.getByRole("button", { name: "Принять VEX" })).toHaveAccessibleDescription(reason);
  });

  it("все действия и «Снять выделение» достижимы по Tab", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<BulkActionBar count={2} actions={ACTIONS} onClear={vi.fn()} />);
    await user.tab();
    await user.tab();

    // Act
    await user.tab();

    // Assert
    expect(screen.getByRole("button", { name: CLEAR })).toHaveFocus();
  });

  it("«Снять выделение» вызывает onClear", async () => {
    // Arrange
    const user = userEvent.setup();
    const onClear = vi.fn();
    render(<BulkActionBar count={2} actions={ACTIONS} onClear={onClear} />);

    // Act
    await user.click(screen.getByRole("button", { name: CLEAR }));

    // Assert
    expect(onClear).toHaveBeenCalledTimes(1);
  });

  it("число выбранных объявляется в live-регионе", async () => {
    // Arrange
    const count = 3;

    // Act
    render(
      <>
        <LiveRegion />
        <BulkActionBar count={count} actions={ACTIONS} onClear={vi.fn()} />
      </>,
    );

    // Assert
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Выбрано 3"));
  });

  it("при изменении count объявляется новое число", async () => {
    // Arrange
    const { rerender } = render(
      <>
        <LiveRegion />
        <BulkActionBar count={3} actions={ACTIONS} onClear={vi.fn()} />
      </>,
    );
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Выбрано 3"));

    // Act
    rerender(
      <>
        <LiveRegion />
        <BulkActionBar count={4} actions={ACTIONS} onClear={vi.fn()} />
      </>,
    );

    // Assert
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Выбрано 4"));
  });

  it("перерисовка с тем же count не объявляет повторно", async () => {
    // Arrange
    const { rerender } = render(
      <>
        <LiveRegion />
        <BulkActionBar count={3} actions={ACTIONS} onClear={vi.fn()} />
      </>,
    );
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Выбрано 3"));

    // Act
    rerender(
      <>
        <LiveRegion />
        <BulkActionBar count={3} actions={[...ACTIONS]} onClear={vi.fn()} />
      </>,
    );

    // Assert
    expect(screen.getByRole("status")).toHaveTextContent("Выбрано 3");
  });

  it("пока панель видна, документ резервирует scroll-padding-bottom по её высоте (WCAG 2.4.11)", () => {
    // Arrange
    const height = 48;
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      height,
    } as DOMRect);

    // Act
    render(<BulkActionBar count={2} actions={ACTIONS} onClear={vi.fn()} />);

    // Assert
    expect(document.documentElement.style.scrollPaddingBottom).toBe("80px");
  });

  it("при count 0 резерв scroll-padding-bottom снимается", () => {
    // Arrange
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      height: 48,
    } as DOMRect);
    const { rerender } = render(<BulkActionBar count={2} actions={ACTIONS} onClear={vi.fn()} />);

    // Act
    rerender(<BulkActionBar count={0} actions={ACTIONS} onClear={vi.fn()} />);

    // Assert
    expect(document.documentElement.style.scrollPaddingBottom).toBe("");
  });

  it("при размонтировании резерв снимается", () => {
    // Arrange
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      height: 48,
    } as DOMRect);
    const { unmount } = render(<BulkActionBar count={2} actions={ACTIONS} onClear={vi.fn()} />);

    // Act
    unmount();

    // Assert
    expect(document.documentElement.style.scrollPaddingBottom).toBe("");
  });
});
