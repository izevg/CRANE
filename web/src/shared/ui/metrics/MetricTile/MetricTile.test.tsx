// biome-ignore-all lint/style/noMagicNumbers: тестовые данные плиток — произвольные числа
import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { MetricTile, MetricTileSkeleton } from "~/shared/ui/metrics";

// Intl разделяет группы неразрывным пробелом; нормализатор Testing Library по умолчанию свернул бы его.
const RAW = { normalizer: (text: string) => text };
const DASH = "—";
const NO_DATA = "Нет данных";
const TONE_LABEL = "требует действия";
const STALE_TEXT = /обновлено/;
const DELTA_TEXT = /Изменение:/;
const MINUTES_AGO = 3 * 60 * 1000;

describe("MetricTile", () => {
  it("значение null показывает прочерк и «Нет данных», а не 0", () => {
    // Arrange
    const value: null = null;

    // Act
    render(<MetricTile label="Версий" value={value} />);

    // Assert
    expect(screen.getByText(DASH)).toBeInTheDocument();
    expect(screen.getByText(NO_DATA)).toBeInTheDocument();
    expect(screen.queryByText("0")).toBeNull();
  });

  it("число форматируется с разделителем групп", () => {
    // Arrange
    const value = 48102;

    // Act
    render(<MetricTile label="Пакетов" value={value} />);

    // Assert
    expect(screen.getByText("48\u00a0102", RAW)).toBeInTheDocument();
  });

  it("усечённый счётчик показывается со знаком «+»", () => {
    // Arrange
    const value = { count: 1000, capped: true };

    // Act
    render(<MetricTile label="Находок" value={value} />);

    // Assert
    expect(screen.getByText("1\u00a0000+", RAW)).toBeInTheDocument();
  });

  it("точный счётчик без «+»", () => {
    // Arrange
    const value = { count: 1000, capped: false };

    // Act
    render(<MetricTile label="Находок" value={value} />);

    // Assert
    expect(screen.getByText("1\u00a0000", RAW)).toBeInTheDocument();
  });

  it("текстовое значение выводится как есть", () => {
    // Arrange
    const value = "Деградация";

    // Act
    render(<MetricTile label="Статус" value={value} />);

    // Assert
    expect(screen.getByText(value)).toBeInTheDocument();
  });

  it("с href вся плитка — одна ссылка с именем «метка: значение, подпись тона»", () => {
    // Arrange
    const props = { label: "P0 открыто", value: 1, tone: "error", toneLabel: TONE_LABEL } as const;

    // Act
    render(<MetricTile {...props} href="/findings?priority=P0" />);

    // Assert
    const link = screen.getByRole("link", { name: "P0 открыто: 1, требует действия" });
    expect(link).toHaveAttribute("href", "/findings?priority=P0");
    expect(screen.getAllByRole("link")).toHaveLength(1);
  });

  it("плитка-ссылка с нейтральным тоном называется «метка: значение»", () => {
    // Arrange
    const label = "Версий";

    // Act
    render(<MetricTile label={label} value={48} href="/versions" />);

    // Assert
    expect(screen.getByRole("link", { name: "Версий: 48" })).toBeInTheDocument();
  });

  it("без href плитка не ссылка, а именованная группа", () => {
    // Arrange
    const label = "Версий";

    // Act
    render(<MetricTile label={label} value={48} />);

    // Assert
    expect(screen.queryByRole("link")).toBeNull();
    expect(screen.getByRole("group", { name: label })).toBeInTheDocument();
  });

  it("тон error показывает toneLabel текстом", () => {
    // Arrange
    const props = { label: "P0 открыто", value: 1, tone: "error", toneLabel: TONE_LABEL } as const;

    // Act
    render(<MetricTile {...props} />);

    // Assert
    const group = screen.getByRole("group", { name: "P0 открыто" });
    expect(within(group).getByText(TONE_LABEL)).toBeInTheDocument();
    expect(group).toHaveAttribute("data-tone", "error");
  });

  it("тон warning показывает toneLabel текстом", () => {
    // Arrange
    const props = { label: "Очередь", value: 9, tone: "warning", toneLabel: "растёт" } as const;

    // Act
    render(<MetricTile {...props} />);

    // Assert
    expect(screen.getByText("растёт")).toBeInTheDocument();
  });

  it("нейтральная плитка не показывает бейдж тона", () => {
    // Arrange
    const label = "Версий";

    // Act
    render(<MetricTile label={label} value={48} />);

    // Assert
    expect(screen.getByRole("group", { name: label })).not.toHaveAttribute("data-tone");
  });

  it("при ошибке опроса значение остаётся, а рядом — «обновлено … назад»", () => {
    // Arrange
    const stale = { updatedAt: new Date(Date.now() - MINUTES_AGO).toISOString() };

    // Act
    render(<MetricTile label="Версий" value={48} stale={stale} />);

    // Assert
    expect(screen.getByText("48")).toBeInTheDocument();
    expect(screen.getByText(STALE_TEXT)).toHaveTextContent("обновлено 3 мин назад");
  });

  it("дельта выводится со словом «Изменение» для скринридера и знаком минус", () => {
    // Arrange
    const delta = { pct: -3.1, polarity: "down-good" } as const;

    // Act
    render(<MetricTile label="Задержка" value={84} delta={delta} />);

    // Assert
    expect(screen.getByText(DELTA_TEXT).parentElement).toHaveTextContent("Изменение: −3,1");
  });

  it("caption показывается, когда дельты нет", () => {
    // Arrange
    const caption = "старейшая 3 мин";

    // Act
    render(<MetricTile label="Очередь" value={2} caption={caption} />);

    // Assert
    expect(screen.getByText(caption)).toBeInTheDocument();
  });

  it("спарклайн скрыт от скринридера", () => {
    // Arrange
    const spark = [1, 2, 3];

    // Act
    render(<MetricTile label="Очередь" value={2} spark={spark} />);

    // Assert
    expect(screen.queryByRole("img")).toBeNull();
    expect(
      screen.getByRole("group", { name: "Очередь" }).querySelector("svg[aria-hidden]"),
    ).not.toBeNull();
  });
});

describe("MetricTileSkeleton", () => {
  it("помечен как занятый и не содержит текста", () => {
    // Arrange
    const label = "загрузка";

    // Act
    const { container } = render(<MetricTileSkeleton className={label} />);

    // Assert
    expect(container.firstElementChild).toHaveAttribute("aria-busy", "true");
    expect(container).toHaveTextContent("");
  });
});
