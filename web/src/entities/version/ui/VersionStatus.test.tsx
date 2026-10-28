import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import type { BlockReason, VersionSummary } from "~/shared/api";
import { VersionStatus } from "../index";

const REASON_TEXT = /Экстренно|По правилу/;
const FIXED_IN = /Исправлено в/;
const CVE = /CVE-2026-31337/;

const view = (
  state: VersionSummary["state"],
  block_reason: BlockReason | null = null,
): Pick<VersionSummary, "state" | "block_reason"> => ({ state, block_reason });

const BLOCK_ID = "0193a7c2-5e4d-7b3a-8c1f-2d9e6a4b7c10";
const EMERGENCY = { kind: "emergency", block_id: BLOCK_ID, advisory_id: "CVE-2026-31337" } as const;

describe("VersionStatus", () => {
  it("подпись состояния берётся из словаря, а не из кода API", () => {
    // Arrange
    const version = view("quarantined");

    // Act
    render(<VersionStatus version={version} />);

    // Assert
    expect(screen.getByText("Карантин")).toBeInTheDocument();
    expect(screen.queryByText("quarantined")).not.toBeInTheDocument();
  });

  it("у заблокированной экстренно причина стоит рядом с advisory", () => {
    // Arrange
    const version = view("blocked", {
      kind: "emergency",
      block_id: "0193a7c2-5e4d-7b3a-8c1f-2d9e6a4b7c10",
      advisory_id: "CVE-2026-31337",
    });

    // Act
    render(<VersionStatus version={version} />);

    // Assert
    expect(screen.getByText("Заблокирована")).toBeInTheDocument();
    expect(screen.getByText("Экстренно · CVE-2026-31337")).toBeInTheDocument();
  });

  it("у заблокированной по правилу причина — правило", () => {
    // Arrange
    const version = view("blocked", { kind: "rule", rule_id: "policy.deny.unlicensed" });

    // Act
    render(<VersionStatus version={version} />);

    // Assert
    expect(screen.getByText("По правилу · policy.deny.unlicensed")).toBeInTheDocument();
  });

  it("исправленная версия показана видимым текстом рядом с причиной", () => {
    // Arrange
    const version = view("blocked", {
      kind: "emergency",
      block_id: "0193a7c2-5e4d-7b3a-8c1f-2d9e6a4b7c10",
      advisory_id: "CVE-2026-31337",
      fixed_in: "4.17.21",
    });

    // Act
    render(<VersionStatus version={version} />);

    // Assert
    expect(screen.getByText("Исправлено в 4.17.21")).toBeVisible();
  });

  it("без fixed_in текста про исправление нет", () => {
    // Arrange
    const version = view("blocked", {
      kind: "emergency",
      block_id: "0193a7c2-5e4d-7b3a-8c1f-2d9e6a4b7c10",
      advisory_id: "CVE-2026-31337",
    });

    // Act
    render(<VersionStatus version={version} />);

    // Assert
    expect(screen.queryByText(FIXED_IN)).not.toBeInTheDocument();
  });

  it("заблокированная без причины показывает только состояние", () => {
    // Arrange
    const version = view("blocked");

    // Act
    render(<VersionStatus version={version} />);

    // Assert
    expect(screen.getByText("Заблокирована")).toBeInTheDocument();
    expect(screen.queryByText(REASON_TEXT)).not.toBeInTheDocument();
  });

  it("причина не показывается у состояния, отличного от blocked", () => {
    // Arrange
    const version = view("released", {
      kind: "emergency",
      block_id: "0193a7c2-5e4d-7b3a-8c1f-2d9e6a4b7c10",
      advisory_id: "CVE-2026-31337",
    });

    // Act
    render(<VersionStatus version={version} />);

    // Assert
    expect(screen.queryByText(CVE)).not.toBeInTheDocument();
  });

  it("тон бейджа соответствует состоянию: заблокирована — error", () => {
    // Arrange
    const version = view("blocked");

    // Act
    render(<VersionStatus version={version} />);

    // Assert
    expect(screen.getByText("Заблокирована").closest("[data-tone]")).toHaveAttribute(
      "data-tone",
      "error",
    );
  });

  it("идентификатор экстренной блокировки со ссылкой, если страница передала blockHref", () => {
    // Arrange
    const version = view("blocked", EMERGENCY);

    // Act
    render(<VersionStatus version={version} blockHref="/emergency-blocks/0193a7c2" />);

    // Assert
    expect(screen.getByRole("link", { name: `Блокировка ${BLOCK_ID}` })).toHaveAttribute(
      "href",
      "/emergency-blocks/0193a7c2",
    );
  });

  it("без blockHref идентификатор блокировки — копируемое значение с полным id в имени", () => {
    // Arrange
    const version = view("blocked", EMERGENCY);

    // Act
    render(<VersionStatus version={version} />);

    // Assert
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", {
        name: `Скопировать идентификатор блокировки: ${BLOCK_ID}`,
      }),
    ).toBeInTheDocument();
  });

  it("у блокировки по правилу идентификатора блокировки нет", () => {
    // Arrange
    const version = view("blocked", { kind: "rule", rule_id: "policy.deny.unlicensed" });

    // Act
    render(<VersionStatus version={version} blockHref="/emergency-blocks/x" />);

    // Assert
    expect(screen.queryByRole("link")).not.toBeInTheDocument();
    expect(screen.queryByRole("button")).not.toBeInTheDocument();
  });
});
