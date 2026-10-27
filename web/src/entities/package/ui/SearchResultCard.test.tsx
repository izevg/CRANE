import { render, screen, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { SearchItem } from "~/shared/api";
import { BASE_ITEM } from "./fixtures";
import { SearchResultCard } from "./SearchResultCard";

const PILL = /^P\d · /;
const SQUAT = /отличается на 1 символ/;
const ANY_DOWNLOADS = /загрузок за 30 дн/;
const DOWNLOADS = /12\s?340 загрузок за 30 дн/;

function card(item: Partial<SearchItem> = {}, typosquatHref?: string) {
  render(
    <SearchResultCard
      item={{ ...BASE_ITEM, ...item }}
      href="/packages/acme-ui"
      {...(typosquatHref === undefined ? {} : { typosquatHref })}
    />,
  );
}

describe("SearchResultCard", () => {
  it("без risk пилюли риска нет", () => {
    // Arrange
    // Act
    card();

    // Assert
    expect(screen.queryByText(PILL)).not.toBeInTheDocument();
    expect(screen.queryByText("Чисто")).not.toBeInTheDocument();
  });

  it("с risk: null пилюли тоже нет", () => {
    // Arrange
    // Act
    card({ risk: null });

    // Assert
    expect(screen.queryByText(PILL)).not.toBeInTheDocument();
  });

  it("с risk пилюля называет приоритет и одну находку в единственном числе", () => {
    // Arrange
    // Act
    card({ risk: { max_open_priority: "P1", open_count: 1 } });

    // Assert
    expect(screen.getByText("P1 · 1 находка")).toBeVisible();
  });

  it("две находки склоняются как «находки»", () => {
    // Arrange
    // Act
    card({ risk: { max_open_priority: "P2", open_count: 2 } });

    // Assert
    expect(screen.getByText("P2 · 2 находки")).toBeVisible();
  });

  it("пилюля P0 рисуется с заполненной точкой вместо иконки", () => {
    // Arrange
    // Act
    card({ risk: { max_open_priority: "P0", open_count: 1 } });

    // Assert
    const pill = screen.getByText("P0 · 1 находка");
    expect(pill.querySelector('[data-slot="status-dot"]')).not.toBeNull();
    expect(pill.querySelector('[data-slot="status-icon"]')).toBeNull();
  });

  it("пилюля P1 рисуется с иконкой, как и положено тону error", () => {
    // Arrange
    // Act
    card({ risk: { max_open_priority: "P1", open_count: 1 } });

    // Assert
    expect(
      screen.getByText("P1 · 1 находка").querySelector('[data-slot="status-icon"]'),
    ).not.toBeNull();
  });

  it("пять находок склоняются как «находок»", () => {
    // Arrange
    // Act
    card({ risk: { max_open_priority: "P2", open_count: 5 } });

    // Assert
    expect(screen.getByText("P2 · 5 находок")).toBeVisible();
  });

  it("точное совпадение подписано оверлайном", () => {
    // Arrange
    // Act
    card({ exact_match: true });

    // Assert
    expect(screen.getByText("Точное совпадение")).toBeVisible();
  });

  it("без exact_match оверлайна нет", () => {
    // Arrange
    // Act
    card();

    // Assert
    expect(screen.queryByText("Точное совпадение")).not.toBeInTheDocument();
  });

  it("typosquat: предупреждение с расстоянием и ссылка на настоящий пакет", () => {
    // Arrange
    // Act
    card(
      {
        typosquat: {
          similar_to: "@acme/ui",
          similar_to_purl: "pkg:npm/%40acme/ui",
          distance: 1,
        },
        _links: {
          self: { href: "/api/packages/acme-ui" },
          "crane:similar-to": { href: "/packages/npm/npm-hosted/%40acme%2Fui" },
        },
      },
      "/packages/real-acme-ui",
    );

    // Assert
    expect(screen.getByText(SQUAT)).toBeVisible();
    expect(screen.getAllByRole("link").map((link) => link.getAttribute("href"))).toEqual([
      "/packages/real-acme-ui",
      "/packages/acme-ui",
    ]);
  });

  it("typosquat без typosquatHref называет пакет без ссылки", () => {
    // Arrange
    // Act
    card({
      typosquat: {
        similar_to: "@acme/ui",
        similar_to_purl: "pkg:npm/%40acme/ui",
        distance: 1,
      },
    });

    // Assert
    expect(screen.getByText(SQUAT)).toBeVisible();
    expect(screen.getAllByRole("link")).toHaveLength(1);
  });

  it("ссылка в карточке одна — на имени пакета", () => {
    // Arrange
    // Act
    card({ exact_match: true, risk: { max_open_priority: "P1", open_count: 1 } });

    // Assert
    const links = within(screen.getByRole("article")).getAllByRole("link");
    expect(links).toHaveLength(1);
    expect(links[0]).toHaveAttribute("href", "/packages/acme-ui");
    expect(links[0]).toHaveTextContent("@acme/ui");
  });

  it("устаревший пакет получает бейдж «Устарел»", () => {
    // Arrange
    // Act
    card({ lifecycle: { state: "deprecated" } });

    // Assert
    expect(screen.getByText("Устарел")).toBeVisible();
  });

  it("мета: версия, состояние и загрузки", () => {
    // Arrange
    // Act
    card();

    // Assert
    expect(screen.getByText("1.4.2")).toBeVisible();
    expect(screen.getByText("Выпущена")).toBeVisible();
    expect(screen.getByText(DOWNLOADS)).toBeVisible();
  });

  it("поля, которых нет у пакета, не рисуются", () => {
    // Arrange
    // Act
    card({ description: null, latest: null, downloads_30d: null, owner_team: null });

    // Assert
    expect(screen.queryByText("Последняя")).not.toBeInTheDocument();
    expect(screen.queryByText(ANY_DOWNLOADS)).not.toBeInTheDocument();
  });
});
