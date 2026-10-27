import { render, screen } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { Tone } from "~/shared/ui/primitives";
import { StatusBadge } from "~/shared/ui/status";
import { stubMatchMedia } from "~test/matchMedia";

const REDUCED = "(prefers-reduced-motion: reduce)";

// Иконка помечена data-slot="status-icon", точка — "status-dot": класс не проверяем, проверяем наличие глифа.
const WITH_ICON: Tone[] = ["warning", "error"];
const WITH_DOT: Tone[] = ["success", "info", "neutral"];

afterEach(() => {
  vi.unstubAllGlobals();
});

describe("StatusBadge", () => {
  it("показывает обязательную подпись", () => {
    // Arrange
    const label = "Карантин";

    // Act
    render(<StatusBadge tone="warning" label={label} />);

    // Assert
    expect(screen.getByText(label)).toBeInTheDocument();
  });

  it.each(WITH_ICON)("тон %s передаётся иконкой, а не только цветом (WCAG 1.4.1)", (tone) => {
    // Arrange
    const label = "Статус";

    // Act
    const { container } = render(<StatusBadge tone={tone} label={label} />);

    // Assert
    expect(container.querySelector('[data-slot="status-icon"]')).not.toBeNull();
    expect(container.querySelector('[data-slot="status-dot"]')).toBeNull();
  });

  it.each(WITH_DOT)("тон %s показывается точкой", (tone) => {
    // Arrange
    const label = "Статус";

    // Act
    const { container } = render(<StatusBadge tone={tone} label={label} />);

    // Assert
    expect(container.querySelector('[data-slot="status-dot"]')).not.toBeNull();
    expect(container.querySelector('[data-slot="status-icon"]')).toBeNull();
  });

  it("глиф скрыт от скринридера: смысл несёт подпись", () => {
    // Arrange
    const label = "Заблокирована";

    // Act
    const { container } = render(<StatusBadge tone="error" label={label} />);

    // Assert
    expect(
      container.querySelector('[data-slot="status-icon"]')?.closest('[aria-hidden="true"]'),
    ).not.toBeNull();
  });

  it("pulse включает пульсацию", () => {
    // Arrange
    stubMatchMedia();

    // Act
    const { container } = render(<StatusBadge tone="info" label="Идёт проверка" pulse />);

    // Assert
    expect(container.querySelector('[data-pulse="true"]')).not.toBeNull();
  });

  it("без pulse глиф статичен", () => {
    // Arrange
    stubMatchMedia();

    // Act
    const { container } = render(<StatusBadge tone="info" label="Идёт проверка" />);

    // Assert
    expect(container.querySelector("[data-pulse]")).toBeNull();
  });

  it("при prefers-reduced-motion пульсация выключена", () => {
    // Arrange
    stubMatchMedia([REDUCED]);

    // Act
    const { container } = render(<StatusBadge tone="info" label="Идёт проверка" pulse />);

    // Assert
    expect(container.querySelector("[data-pulse]")).toBeNull();
  });
});
