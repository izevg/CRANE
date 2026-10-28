import { msg } from "@lingui/core/macro";
import { describe, expect, it } from "vitest";
import { documentTitle, pageTitleOf } from "./title";

const NOT_FOUND = msg`Не найдено или нет доступа`;
const STATUS = msg`Статус системы`;
const TOKENS = msg`Токены`;

describe("pageTitleOf", () => {
  it("берёт заголовок самого глубокого совпавшего маршрута", () => {
    // Arrange
    const matches = [
      { status: "success", staticData: {} },
      { status: "success", staticData: { title: STATUS } },
    ];

    // Act
    const title = pageTitleOf(matches, NOT_FOUND);

    // Assert
    expect(title).toBe(STATUS);
  });

  it("пропускает маршруты без заголовка, но не берёт заголовок родителя поверх дочернего", () => {
    // Arrange
    const matches = [
      { status: "success", staticData: { title: TOKENS } },
      { status: "success", staticData: { title: STATUS } },
      { status: "success" },
    ];

    // Act
    const title = pageTitleOf(matches, NOT_FOUND);

    // Assert
    expect(title).toBe(STATUS);
  });

  it("loader бросил notFound — заголовок «не найдено», а не заголовок страницы", () => {
    // Arrange
    const matches = [{ status: "notFound", staticData: { title: STATUS } }];

    // Act
    const title = pageTitleOf(matches, NOT_FOUND);

    // Assert
    expect(title).toBe(NOT_FOUND);
  });

  it("notFound в одном из совпадений достаточно, остальные совпадения успешны", () => {
    // Arrange
    const matches = [
      { status: "success", staticData: { title: TOKENS } },
      { status: "notFound", staticData: { title: STATUS } },
      { status: "success" },
    ];

    // Act
    const title = pageTitleOf(matches, NOT_FOUND);

    // Assert
    expect(title).toBe(NOT_FOUND);
  });

  it("адрес без маршрута (globalNotFound) — тот же заголовок «не найдено»", () => {
    // Arrange
    const matches = [{ status: "success", globalNotFound: true }];

    // Act
    const title = pageTitleOf(matches, NOT_FOUND);

    // Assert
    expect(title).toBe(NOT_FOUND);
  });

  it("нет заголовков — undefined", () => {
    // Arrange
    const matches = [{ status: "success" }];

    // Act
    const title = pageTitleOf(matches, NOT_FOUND);

    // Assert
    expect(title).toBeUndefined();
  });
});

describe("documentTitle", () => {
  it("«<Страница> — CRANE»", () => {
    // Arrange
    const page = "Статус системы";

    // Act
    const title = documentTitle(page);

    // Assert
    expect(title).toBe("Статус системы — CRANE");
  });

  it("без страницы только имя продукта", () => {
    // Arrange
    const page = undefined;

    // Act
    const title = documentTitle(page);

    // Assert
    expect(title).toBe("CRANE");
  });
});
