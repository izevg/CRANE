import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { announce } from "~/shared/lib/a11y";
import { type Banner, BannerStack } from "./BannerStack";

vi.mock("~/shared/lib/a11y", () => ({ announce: vi.fn() }));

beforeEach(() => {
  vi.mocked(announce).mockClear();
});

const MORE = /Ещё/;
const COLLAPSE = /Свернуть/;

const banner = (id: string, patch: Partial<Banner> = {}): Banner => ({
  id,
  tone: "warning",
  text: `Сообщение ${id}`,
  ...patch,
});

const four = [banner("a"), banner("b"), banner("c"), banner("d")];

describe("BannerStack", () => {
  it("без баннеров ничего не рендерит", () => {
    // Arrange
    // Act
    const { container } = render(<BannerStack banners={[]} />);

    // Assert
    expect(container).toBeEmptyDOMElement();
  });

  it("при четырёх баннерах видны два, остальные свёрнуты в «Ещё 2»", () => {
    // Arrange
    // Act
    render(<BannerStack banners={four} />);

    // Assert
    expect(screen.getByText("Сообщение a")).toBeInTheDocument();
    expect(screen.getByText("Сообщение b")).toBeInTheDocument();
    expect(screen.queryByText("Сообщение c")).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Ещё 2" })).toHaveAttribute("aria-expanded", "false");
  });

  it("два баннера показываются без кнопки «Ещё»", () => {
    // Arrange
    // Act
    render(<BannerStack banners={four.slice(0, 2)} />);

    // Assert
    expect(screen.queryByRole("button", { name: MORE })).not.toBeInTheDocument();
  });

  it("«Ещё 2» раскрывает все баннеры", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<BannerStack banners={four} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Ещё 2" }));

    // Assert
    expect(screen.getByText("Сообщение c")).toBeInTheDocument();
    expect(screen.getByText("Сообщение d")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Скрыть остальные" })).toHaveAttribute(
      "aria-expanded",
      "true",
    );
  });

  it("повторное нажатие снова сворачивает лишние баннеры", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<BannerStack banners={four} />);
    await user.click(screen.getByRole("button", { name: "Ещё 2" }));

    // Act
    await user.click(screen.getByRole("button", { name: "Скрыть остальные" }));

    // Assert
    expect(screen.queryByText("Сообщение c")).not.toBeInTheDocument();
  });

  it("роль alert только у срочного (P0) баннера, обычные без роли", () => {
    // Arrange
    const banners = [banner("p0", { tone: "error", urgent: true }), banner("w")];

    // Act
    render(<BannerStack banners={banners} />);

    // Assert
    expect(screen.getByRole("alert")).toHaveTextContent("Сообщение p0");
    expect(screen.queryByRole("status")).not.toBeInTheDocument();
  });

  it("действие баннера рендерится ссылкой только когда передано", () => {
    // Arrange
    const withAction = banner("a", { action: { label: "Подробнее", run: { href: "/status" } } });

    // Act
    render(<BannerStack banners={[withAction, banner("b")]} />);

    // Assert
    expect(screen.getAllByRole("link")).toHaveLength(1);
    expect(screen.getByRole("link", { name: "Подробнее" })).toHaveAttribute("href", "/status");
  });

  it("warning и error не закрываются: кнопки «Свернуть» нет", () => {
    // Arrange
    // Act
    render(
      <BannerStack banners={[banner("w"), banner("e", { tone: "error" })]} onCollapse={vi.fn()} />,
    );

    // Assert
    expect(screen.queryByRole("button", { name: COLLAPSE })).not.toBeInTheDocument();
  });

  it("info сворачивается кнопкой и сообщает id баннера", async () => {
    // Arrange
    const user = userEvent.setup();
    const onCollapse = vi.fn();
    render(<BannerStack banners={[banner("i", { tone: "info" })]} onCollapse={onCollapse} />);

    // Act
    await user.click(screen.getByRole("button", { name: COLLAPSE }));

    // Assert
    expect(onCollapse).toHaveBeenCalledWith("i");
  });

  it("info без обработчика не получает кнопку «Свернуть»", () => {
    // Arrange
    // Act
    render(<BannerStack banners={[banner("i", { tone: "info" })]} />);

    // Assert
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });

  describe("объявление новых баннеров", () => {
    it("новый обычный баннер объявляется один раз его текстом", () => {
      // Arrange
      // Act
      render(<BannerStack banners={[banner("a")]} />);

      // Assert
      expect(announce).toHaveBeenCalledTimes(1);
      expect(announce).toHaveBeenCalledWith("Сообщение a");
    });

    it("повторный рендер с тем же набором не объявляет", () => {
      // Arrange
      const banners = [banner("a")];
      const { rerender } = render(<BannerStack banners={banners} />);
      vi.mocked(announce).mockClear();

      // Act
      rerender(<BannerStack banners={[banner("a")]} />);

      // Assert
      expect(announce).not.toHaveBeenCalled();
    });

    it("появление нового id объявляет только его", () => {
      // Arrange
      const { rerender } = render(<BannerStack banners={[banner("a")]} />);
      vi.mocked(announce).mockClear();

      // Act
      rerender(<BannerStack banners={[banner("a"), banner("b")]} />);

      // Assert
      expect(announce).toHaveBeenCalledTimes(1);
      expect(announce).toHaveBeenCalledWith("Сообщение b");
    });

    it("раскрытие «Ещё N» ничего не объявляет", async () => {
      // Arrange
      const user = userEvent.setup();
      render(<BannerStack banners={four} />);
      vi.mocked(announce).mockClear();

      // Act
      await user.click(screen.getByRole("button", { name: MORE }));

      // Assert
      expect(announce).not.toHaveBeenCalled();
    });

    it("срочный баннер не дублируется в announce: его объявляет role=alert", () => {
      // Arrange
      // Act
      render(<BannerStack banners={[banner("p0", { tone: "error", urgent: true })]} />);

      // Assert
      expect(announce).not.toHaveBeenCalled();
      expect(screen.getByRole("alert")).toHaveTextContent("Сообщение p0");
    });
  });
});
