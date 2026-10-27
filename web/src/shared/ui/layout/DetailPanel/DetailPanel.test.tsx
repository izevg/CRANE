// biome-ignore-all lint/style/useComponentExportOnlyModules: тестовый компонент-помощник не экспортируется
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { useCallback, useRef, useState } from "react";
import { describe, expect, it, vi } from "vitest";
import type { Action } from "~/shared/ui/actions";
import { DetailPanel, type PageActions } from "~/shared/ui/layout";
import { stubMatchMedia } from "~test/matchMedia";

const DOCKED_QUERY = "(min-width: 1536px)";

function Harness({ footer, menu }: { footer?: PageActions; menu?: readonly Action[] }) {
  const [open, setOpen] = useState(false);
  const rowRef = useRef<HTMLButtonElement>(null);
  const show = useCallback(() => setOpen(true), []);
  return (
    <>
      <button type="button" ref={rowRef} onClick={show}>
        {"Строка left-pad"}
      </button>
      <DetailPanel
        open={open}
        onOpenChange={setOpen}
        title="left-pad@1.3.0"
        status={<span>{"Карантин"}</span>}
        returnFocusRef={rowRef}
        {...(footer ? { footer } : {})}
        {...(menu ? { menu } : {})}
      >
        <p>{"Содержимое панели"}</p>
      </DetailPanel>
    </>
  );
}

describe("DetailPanel", () => {
  describe("широкий экран (docked)", () => {
    it("рендерится немодальной областью complementary с именем из заголовка", async () => {
      // Arrange
      stubMatchMedia([DOCKED_QUERY]);
      const user = userEvent.setup();
      render(<Harness />);

      // Act
      await user.click(screen.getByRole("button", { name: "Строка left-pad" }));

      // Assert
      expect(screen.getByRole("complementary", { name: "left-pad@1.3.0" })).toBeInTheDocument();
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    });

    it("открытие переводит фокус на заголовок панели", async () => {
      // Arrange
      stubMatchMedia([DOCKED_QUERY]);
      const user = userEvent.setup();
      render(<Harness />);

      // Act
      await user.click(screen.getByRole("button", { name: "Строка left-pad" }));

      // Assert
      expect(screen.getByRole("heading", { level: 2, name: "left-pad@1.3.0" })).toHaveFocus();
    });

    it("Esc закрывает панель и возвращает фокус на строку", async () => {
      // Arrange
      stubMatchMedia([DOCKED_QUERY]);
      const user = userEvent.setup();
      render(<Harness />);
      await user.click(screen.getByRole("button", { name: "Строка left-pad" }));

      // Act
      await user.keyboard("{Escape}");

      // Assert
      expect(screen.queryByRole("complementary")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Строка left-pad" })).toHaveFocus();
    });

    it("кнопка «Закрыть» закрывает панель и возвращает фокус на строку", async () => {
      // Arrange
      stubMatchMedia([DOCKED_QUERY]);
      const user = userEvent.setup();
      render(<Harness />);
      await user.click(screen.getByRole("button", { name: "Строка left-pad" }));

      // Act
      await user.click(screen.getByRole("button", { name: "Закрыть" }));

      // Assert
      expect(screen.queryByRole("complementary")).not.toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Строка left-pad" })).toHaveFocus();
    });

    it("Esc в меню «…» закрывает меню, а не панель", async () => {
      // Arrange
      stubMatchMedia([DOCKED_QUERY]);
      const user = userEvent.setup();
      render(<Harness menu={[{ label: "Открыть полностью", run: { href: "/p/left-pad" } }]} />);
      await user.click(screen.getByRole("button", { name: "Строка left-pad" }));
      await user.click(screen.getByRole("button", { name: "Другие действия" }));
      await screen.findByRole("menu");

      // Act
      await user.keyboard("{Escape}");

      // Assert
      await waitFor(() => expect(screen.queryByRole("menu")).not.toBeInTheDocument());
      expect(screen.getByRole("complementary")).toBeInTheDocument();
    });
  });

  describe("узкий экран (Sheet)", () => {
    it("рендерится модальным диалогом с именем из заголовка", async () => {
      // Arrange
      stubMatchMedia();
      const user = userEvent.setup();
      render(<Harness />);

      // Act
      await user.click(screen.getByRole("button", { name: "Строка left-pad" }));

      // Assert
      expect(await screen.findByRole("dialog", { name: "left-pad@1.3.0" })).toBeInTheDocument();
      expect(screen.queryByRole("complementary")).not.toBeInTheDocument();
    });

    it("открытие переводит фокус на заголовок панели", async () => {
      // Arrange
      stubMatchMedia();
      const user = userEvent.setup();
      render(<Harness />);

      // Act
      await user.click(screen.getByRole("button", { name: "Строка left-pad" }));

      // Assert
      const heading = await screen.findByRole("heading", { level: 2, name: "left-pad@1.3.0" });
      await waitFor(() => expect(heading).toHaveFocus());
    });

    it("Esc закрывает панель и возвращает фокус на строку", async () => {
      // Arrange
      stubMatchMedia();
      const user = userEvent.setup();
      render(<Harness />);
      await user.click(screen.getByRole("button", { name: "Строка left-pad" }));
      await screen.findByRole("dialog");

      // Act
      await user.keyboard("{Escape}");

      // Assert
      await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
      expect(screen.getByRole("button", { name: "Строка left-pad" })).toHaveFocus();
    });

    it("кнопка «Закрыть» закрывает панель и возвращает фокус на строку", async () => {
      // Arrange
      stubMatchMedia();
      const user = userEvent.setup();
      render(<Harness />);
      await user.click(screen.getByRole("button", { name: "Строка left-pad" }));
      await screen.findByRole("dialog");

      // Act
      await user.click(screen.getByRole("button", { name: "Закрыть" }));

      // Assert
      await waitFor(() => expect(screen.queryByRole("dialog")).not.toBeInTheDocument());
      expect(screen.getByRole("button", { name: "Строка left-pad" })).toHaveFocus();
    });
  });

  describe("подвал", () => {
    it("действия подвала рендерятся, когда переданы Action", async () => {
      // Arrange
      stubMatchMedia([DOCKED_QUERY]);
      const user = userEvent.setup();
      render(
        <Harness
          footer={{
            primary: { label: "Принять", run: { onSelect: vi.fn() } },
            secondary: [{ label: "Отклонить", run: { onSelect: vi.fn() } }],
          }}
        />,
      );

      // Act
      await user.click(screen.getByRole("button", { name: "Строка left-pad" }));

      // Assert
      expect(screen.getByRole("button", { name: "Принять" })).toBeInTheDocument();
      expect(screen.getByRole("button", { name: "Отклонить" })).toBeInTheDocument();
    });

    it("без действий в панели только кнопка «Закрыть»", async () => {
      // Arrange
      stubMatchMedia([DOCKED_QUERY]);
      const user = userEvent.setup();
      render(<Harness footer={{}} />);

      // Act
      await user.click(screen.getByRole("button", { name: "Строка left-pad" }));

      // Assert
      const panel = screen.getByRole("complementary");
      expect(within(panel).getAllByRole("button")).toHaveLength(1);
    });
  });
});
