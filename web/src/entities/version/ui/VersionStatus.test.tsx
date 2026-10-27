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
    const version = view("blocked", { kind: "emergency", advisory_id: "CVE-2026-31337" });

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
    const version = view("blocked", { kind: "emergency", advisory_id: "CVE-2026-31337" });

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
    const version = view("released", { kind: "emergency", advisory_id: "CVE-2026-31337" });

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
});
