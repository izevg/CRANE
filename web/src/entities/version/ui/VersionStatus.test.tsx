import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import type { BlockReason, VersionSummary } from "~/shared/api";
import { VersionStatus } from "../index";

const REASON_TEXT = /Экстренно|По правилу/;
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

  it("исправленная версия показывается подсказкой при фокусе на причине", async () => {
    // Arrange
    const user = userEvent.setup();
    render(
      <VersionStatus
        version={view("blocked", {
          kind: "emergency",
          advisory_id: "CVE-2026-31337",
          fixed_in: "4.17.21",
        })}
      />,
    );

    // Act
    await user.tab();

    // Assert
    expect((await screen.findAllByText("Исправлено в 4.17.21")).length).toBeGreaterThan(0);
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
