import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import { Stepper, type StepperStep } from "~/shared/ui/layout";
import { stubMatchMedia } from "~test/matchMedia";

const WIDE_QUERY = "(min-width: 1024px)";
const NAV = "Шаги настройки";
const STEP_LINE = "Шаг 0 из 3 · ";

const STEPS: StepperStep[] = [
  { id: "storage", label: "Хранилище", status: "done_by_env", href: "/setup?step=storage" },
  { id: "auth", label: "Учётная запись", status: "todo" },
  { id: "oidc", label: "Единый вход", status: "unavailable", href: "/setup?step=oidc" },
];

describe("Stepper", () => {
  describe("широкий экран", () => {
    it("отдаёт навигацию с именем и нумерованным списком шагов", () => {
      // Arrange
      stubMatchMedia([WIDE_QUERY]);

      // Act
      render(<Stepper label={NAV} steps={STEPS} current="auth" />);

      // Assert
      const nav = screen.getByRole("navigation", { name: NAV });
      expect(nav).toContainElement(screen.getByRole("list"));
      expect(screen.getAllByRole("listitem")).toHaveLength(STEPS.length);
    });

    it("помечает текущий шаг aria-current=step", () => {
      // Arrange
      stubMatchMedia([WIDE_QUERY]);

      // Act
      render(<Stepper label={NAV} steps={STEPS} current="storage" />);

      // Assert
      expect(screen.getByRole("link", { current: "step" })).toHaveTextContent("Хранилище");
    });

    it("помечает текущий шаг без ссылки, остальные не помечены", () => {
      // Arrange
      stubMatchMedia([WIDE_QUERY]);

      // Act
      render(<Stepper label={NAV} steps={STEPS} current="auth" />);

      // Assert
      expect(screen.getByText("Учётная запись").closest("[aria-current='step']")).not.toBeNull();
      expect(screen.queryByRole("link", { current: "step" })).toBeNull();
    });

    it("шаг без href остаётся текстом, а не ссылкой", () => {
      // Arrange
      stubMatchMedia([WIDE_QUERY]);

      // Act
      render(<Stepper label={NAV} steps={STEPS} current="storage" />);

      // Assert
      expect(screen.queryByRole("link", { name: "Учётная запись, не выполнено" })).toBeNull();
      expect(screen.getAllByRole("link")).toHaveLength(2);
    });

    it("статус входит в доступное имя ссылки", () => {
      // Arrange
      stubMatchMedia([WIDE_QUERY]);

      // Act
      render(<Stepper label={NAV} steps={STEPS} current="auth" />);

      // Assert
      expect(screen.getByRole("link", { name: "Хранилище, выполнено окружением" })).toHaveAttribute(
        "href",
        "/setup?step=storage",
      );
    });

    it("не показывает кнопку «Все шаги»", () => {
      // Arrange
      stubMatchMedia([WIDE_QUERY]);

      // Act
      render(<Stepper label={NAV} steps={STEPS} current="auth" />);

      // Assert
      expect(screen.queryByRole("button", { name: "Все шаги" })).toBeNull();
    });
  });

  describe("узкий экран", () => {
    it("показывает строку «Шаг 2 из 3 · название» и свёрнутый список", () => {
      // Arrange
      stubMatchMedia();

      // Act
      render(<Stepper label={NAV} steps={STEPS} current="auth" />);

      // Assert
      expect(screen.getByText("Шаг 2 из 3 · Учётная запись")).toBeVisible();
      expect(screen.getByRole("button", { name: "Все шаги" })).toHaveAttribute(
        "aria-expanded",
        "false",
      );
      expect(screen.queryByRole("list")).toBeNull();
    });

    it("current вне списка: вместо строки «Шаг 0 из N» рисуется только список", () => {
      // Arrange
      stubMatchMedia();

      // Act
      render(<Stepper label={NAV} steps={STEPS} current="missing" />);

      // Assert
      expect(screen.queryByText(STEP_LINE)).toBeNull();
      expect(screen.getAllByRole("listitem")).toHaveLength(STEPS.length);
    });

    it("«Все шаги» раскрывает список", async () => {
      // Arrange
      stubMatchMedia();
      const user = userEvent.setup();
      render(<Stepper label={NAV} steps={STEPS} current="auth" />);

      // Act
      await user.click(screen.getByRole("button", { name: "Все шаги" }));

      // Assert
      expect(screen.getByRole("button", { name: "Все шаги" })).toHaveAttribute(
        "aria-expanded",
        "true",
      );
      expect(screen.getAllByRole("listitem")).toHaveLength(STEPS.length);
    });

    it("повторное нажатие сворачивает список обратно", async () => {
      // Arrange
      stubMatchMedia();
      const user = userEvent.setup();
      render(<Stepper label={NAV} steps={STEPS} current="auth" />);
      await user.click(screen.getByRole("button", { name: "Все шаги" }));

      // Act
      await user.click(screen.getByRole("button", { name: "Все шаги" }));

      // Assert
      expect(screen.getByRole("button", { name: "Все шаги" })).toHaveAttribute(
        "aria-expanded",
        "false",
      );
    });
  });
});
