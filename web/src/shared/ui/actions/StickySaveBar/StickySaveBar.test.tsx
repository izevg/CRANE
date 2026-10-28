import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import { LiveRegion } from "~/shared/lib/a11y";
import { StickySaveBar } from "~/shared/ui/actions";

const SAVE = "Сохранить";
const DISCARD = "Отменить";
const ANNOUNCE_SETTLE_MS = 150;

type BarProps = Parameters<typeof StickySaveBar>[0];

const noFocusTarget = (): HTMLElement | null => null;
const FORM_ID = "settings-form";
const formTarget = () => document.querySelector<HTMLElement>(`#${FORM_ID}`);

function bar(props: Partial<BarProps> = {}) {
  return (
    <>
      <LiveRegion />
      <StickySaveBar
        dirty={3}
        state="idle"
        onSave={vi.fn()}
        onDiscard={vi.fn()}
        returnFocus={noFocusTarget}
        {...props}
      />
    </>
  );
}

afterEach(() => {
  document.documentElement.style.removeProperty("scroll-padding-bottom");
  vi.restoreAllMocks();
});

describe("StickySaveBar", () => {
  it("без правок и без итога ничего не рендерится", () => {
    // Arrange
    const props: Partial<BarProps> = { dirty: 0, state: "idle" };

    // Act
    render(bar(props));

    // Assert
    expect(screen.queryByRole("button")).toBeNull();
  });

  it("показывает число изменённых и обе кнопки", () => {
    // Arrange
    const props: Partial<BarProps> = { dirty: 3 };

    // Act
    render(bar(props));

    // Assert
    expect(screen.getByText("Изменено: 3")).toBeVisible();
    expect(screen.getByRole("button", { name: SAVE })).toBeEnabled();
    expect(screen.getByRole("button", { name: DISCARD })).toBeEnabled();
  });

  it("«Сохранить» вызывает onSave", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(bar({ onSave }));

    // Act
    await user.click(screen.getByRole("button", { name: SAVE }));

    // Assert
    expect(onSave).toHaveBeenCalledTimes(1);
  });

  it("«Отменить» вызывает onDiscard", async () => {
    // Arrange
    const user = userEvent.setup();
    const onDiscard = vi.fn();
    render(bar({ onDiscard }));

    // Act
    await user.click(screen.getByRole("button", { name: DISCARD }));

    // Assert
    expect(onDiscard).toHaveBeenCalledTimes(1);
  });

  it("во время сохранения обе кнопки заблокированы, а «Сохранить» помечена занятой", () => {
    // Arrange
    const props: Partial<BarProps> = { state: "saving" };

    // Act
    render(bar(props));

    // Assert
    expect(screen.getByRole("button", { name: SAVE })).toHaveAttribute("aria-disabled", "true");
    expect(screen.getByRole("button", { name: SAVE })).toHaveAttribute("aria-busy", "true");
    expect(screen.getByRole("button", { name: DISCARD })).toHaveAttribute("aria-disabled", "true");
  });

  it("во время сохранения клик по «Сохранить» не вызывает onSave повторно", async () => {
    // Arrange
    const user = userEvent.setup();
    const onSave = vi.fn();
    render(bar({ state: "saving", onSave }));

    // Act
    await user.click(screen.getByRole("button", { name: SAVE }));

    // Assert
    expect(onSave).not.toHaveBeenCalled();
  });

  it("при частичном сохранении показывает «Сохранено 2 из 3» и остаток правок", () => {
    // Arrange
    const props: Partial<BarProps> = { dirty: 1, state: { saved: 2, total: 3 } };

    // Act
    render(bar(props));

    // Assert
    expect(screen.getByText("Сохранено 2 из 3")).toBeVisible();
    expect(screen.getByText("Изменено: 1")).toBeVisible();
  });

  it("частичный итог отделён от числа правок разделителем", () => {
    // Arrange
    const props: Partial<BarProps> = { dirty: 1, state: { saved: 2, total: 3 } };

    // Act
    render(bar(props));

    // Assert
    const line = screen.getByText("Изменено: 1").closest("p");
    expect(line).toHaveTextContent("Изменено: 1·Сохранено 2 из 3");
    // Разделитель виден и скринридеру: иначе две фразы читаются слитно
    expect(line?.querySelector("[aria-hidden]")).toBeNull();
  });

  it("полный успех объявляется, а панель уходит без текста итога", async () => {
    // Arrange
    const { rerender } = render(bar({ dirty: 3, state: "saving" }));

    // Act
    rerender(bar({ dirty: 0, state: { saved: 3, total: 3 } }));

    // Assert
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Сохранено 3 из 3"));
    await waitFor(() => expect(screen.queryByRole("button", { name: SAVE })).toBeNull());
  });

  it("когда панель уходит, фокус с её кнопки переходит на цель returnFocus", async () => {
    // Arrange
    const user = userEvent.setup();
    const { rerender } = render(
      <>
        <form id={FORM_ID} tabIndex={-1} />
        {bar({ returnFocus: formTarget })}
      </>,
    );
    await user.click(screen.getByRole("button", { name: DISCARD }));

    // Act
    rerender(
      <>
        <form id={FORM_ID} tabIndex={-1} />
        {bar({ dirty: 0, returnFocus: formTarget })}
      </>,
    );

    // Assert
    expect(formTarget()).toHaveFocus();
  });

  it("фокус вне панели при её уходе не трогается", async () => {
    // Arrange
    const user = userEvent.setup();
    const field = <input aria-label="Поле" />;
    const { rerender } = render(
      <>
        <form id={FORM_ID} tabIndex={-1} />
        {field}
        {bar({ returnFocus: formTarget })}
      </>,
    );
    await user.click(screen.getByRole("textbox", { name: "Поле" }));

    // Act
    rerender(
      <>
        <form id={FORM_ID} tabIndex={-1} />
        {field}
        {bar({ dirty: 0, returnFocus: formTarget })}
      </>,
    );

    // Assert
    expect(screen.getByRole("textbox", { name: "Поле" })).toHaveFocus();
  });

  it("первая правка объявляется в live-регионе", async () => {
    // Arrange
    const props: Partial<BarProps> = { dirty: 1 };

    // Act
    render(bar(props));

    // Assert
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Изменено: 1"));
  });

  it("следующие правки той же сессии не объявляются заново", async () => {
    // Arrange
    const { rerender } = render(bar({ dirty: 1 }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Изменено: 1"));

    // Act
    rerender(bar({ dirty: 2 }));

    // Assert
    await new Promise((resolve) => setTimeout(resolve, ANNOUNCE_SETTLE_MS));
    expect(screen.getByRole("status")).toHaveTextContent("Изменено: 1");
  });

  it("итог сохранения объявляется", async () => {
    // Arrange
    const { rerender } = render(bar({ dirty: 3, state: "saving" }));

    // Act
    rerender(bar({ dirty: 1, state: { saved: 2, total: 3 } }));

    // Assert
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Сохранено 2 из 3"));
  });

  it("после сброса правок следующая первая правка объявляется снова", async () => {
    // Arrange
    const { rerender } = render(bar({ dirty: 1 }));
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Изменено: 1"));
    rerender(bar({ dirty: 0 }));

    // Act
    rerender(bar({ dirty: 4 }));

    // Assert
    await waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("Изменено: 4"));
  });

  it("на выходе панель показывает последнее число правок, а не ноль", () => {
    // Arrange
    const { rerender } = render(bar({ dirty: 3 }));

    // Act
    rerender(bar({ dirty: 0 }));

    // Assert
    expect(screen.getByText("Изменено: 3")).toBeInTheDocument();
  });

  it("панель, появившаяся после пустого состояния, тоже уходит с задержкой, а не мгновенно", () => {
    // Arrange
    const { rerender } = render(bar({ dirty: 0 }));
    rerender(bar({ dirty: 2 }));

    // Act
    rerender(bar({ dirty: 0 }));

    // Assert
    expect(screen.getByRole("button", { name: SAVE })).toBeInTheDocument();
  });

  it("новая правка во время выхода оставляет панель на месте", async () => {
    // Arrange
    const { rerender } = render(bar({ dirty: 2 }));
    rerender(bar({ dirty: 0 }));

    // Act
    rerender(bar({ dirty: 1 }));

    // Assert
    await new Promise((resolve) => setTimeout(resolve, ANNOUNCE_SETTLE_MS));
    expect(screen.getByRole("button", { name: SAVE })).toBeEnabled();
  });

  it("после возврата во время выхода панель снова уходит с задержкой", async () => {
    // Arrange
    const { rerender } = render(bar({ dirty: 2 }));
    rerender(bar({ dirty: 0 }));
    rerender(bar({ dirty: 1 }));
    await new Promise((resolve) => setTimeout(resolve, ANNOUNCE_SETTLE_MS));

    // Act
    rerender(bar({ dirty: 0 }));

    // Assert
    expect(screen.getByRole("button", { name: SAVE })).toBeInTheDocument();
  });

  it("пока панель видна, документ резервирует scroll-padding-bottom по её высоте (WCAG 2.4.11)", () => {
    // Arrange
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      height: 56,
    } as DOMRect);

    // Act
    render(bar());

    // Assert
    expect(document.documentElement.style.scrollPaddingBottom).toBe("72px");
  });

  it("когда правок не осталось, панель уходит и резерв снимается", async () => {
    // Arrange
    vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue({
      height: 56,
    } as DOMRect);
    const { rerender } = render(bar({ dirty: 2 }));

    // Act
    rerender(bar({ dirty: 0 }));

    // Assert
    await waitFor(() => expect(screen.queryByRole("button", { name: SAVE })).toBeNull());
    expect(document.documentElement.style.scrollPaddingBottom).toBe("");
  });
});
