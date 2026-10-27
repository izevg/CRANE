// biome-ignore-all lint/style/noMagicNumbers: числа в этом файле — тестовые данные (время, размеры, счётчики)
import { act, render, screen } from "@testing-library/react";
import { toast } from "sonner";
import { afterEach, describe, expect, it } from "vitest";
import { Toaster } from "~/shared/ui/primitives";

const CONTAINER_NAME = /Оповещения/;

afterEach(() => {
  act(() => {
    toast.dismiss();
  });
});

describe("Toaster", () => {
  it("показывает тост и объявляет его как регион с русским именем «Оповещения»", async () => {
    // Arrange
    render(<Toaster />);

    // Act
    act(() => {
      toast("Сохранено");
    });

    // Assert
    expect(await screen.findByText("Сохранено")).toBeInTheDocument();
    expect(screen.getByRole("region", { name: CONTAINER_NAME })).toBeInTheDocument();
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

  it("кнопка закрытия тоста называется по-русски", async () => {
    // Arrange
    render(<Toaster />);

    // Act
    act(() => {
      toast("Сбой", { closeButton: true });
    });

    // Assert
    expect(await screen.findByRole("button", { name: "Закрыть" })).toBeInTheDocument();
  });

  it("у предупреждения и ошибки в тосте есть иконка из общего модуля тона", async () => {
    // Arrange
    render(<Toaster />);

    // Act
    act(() => {
      toast.warning("Деградация");
      toast.error("Недоступен");
    });

    // Assert
    await screen.findByText("Недоступен");
    expect(document.querySelectorAll("[data-sonner-toast] [data-icon] svg")).toHaveLength(2);
  });
});
