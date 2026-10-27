// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые данные (время, размеры, счётчики)
import { act, render, screen } from "@testing-library/react";
import { toast } from "sonner";
import { afterEach, describe, expect, it } from "vitest";
import { Toaster } from "~/shared/ui/primitives";

const NOTIFICATIONS = /Notifications/;

afterEach(() => {
  act(() => {
    toast.dismiss();
  });
});

describe("Toaster", () => {
  it("показывает тост и объявляет его как регион уведомлений", async () => {
    // Arrange
    render(<Toaster />);

    // Act
    act(() => {
      toast("Сохранено");
    });

    // Assert
    expect(await screen.findByText("Сохранено")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: NOTIFICATIONS })).toBeInTheDocument();
  });

  it("показывает не больше трёх тостов одновременно", async () => {
    // Arrange
    render(<Toaster />);

    // Act
    act(() => {
      for (const n of [1, 2, 3, 4, 5]) {
        toast(`Тост ${n}`);
      }
    });

    // Assert
    expect(await screen.findByText("Тост 5")).toBeInTheDocument();
    expect(document.querySelectorAll('[data-sonner-toast][data-visible="true"]')).toHaveLength(3);
  });
});
