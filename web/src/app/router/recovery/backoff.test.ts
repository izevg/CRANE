import { describe, expect, it } from "vitest";
import { createBackoff } from "./backoff";

const T0 = 1_000_000;
const SECOND = 1000;
const PAUSE = { s5: 5000, s10: 10_000, s20: 20_000, s30: 30_000, s60: 60_000 };
const ATTEMPTS = 6;
const FORGOTTEN_AFTER_S = 182;
const STILL_REMEMBERED_S = 179;
const BOUNDARY_S = 180;

describe("createBackoff", () => {
  it("паузы 5, 10, 30, 60 с, дальше 60 с", () => {
    // Arrange
    const backoff = createBackoff();

    // Act
    const delays = Array.from({ length: ATTEMPTS }, (_, i) => backoff.next(T0 + i * SECOND));

    // Assert
    expect(delays).toEqual([PAUSE.s5, PAUSE.s10, PAUSE.s30, PAUSE.s60, PAUSE.s60, PAUSE.s60]);
  });

  it("Retry-After сервера удлиняет паузу, но не сокращает её", () => {
    // Arrange
    const backoff = createBackoff();

    // Act
    const delays = [backoff.next(T0, PAUSE.s20), backoff.next(T0 + SECOND, SECOND)];

    // Assert
    expect(delays).toEqual([PAUSE.s20, PAUSE.s10]);
  });

  it("reset начинает расписание сначала", () => {
    // Arrange
    const backoff = createBackoff();
    backoff.next(T0);
    backoff.next(T0 + SECOND);
    backoff.reset();

    // Act
    const delay = backoff.next(T0 + SECOND * 2);

    // Assert
    expect(delay).toBe(PAUSE.s5);
  });

  it("сбой через долгое время после последнего повтора — новый, паузы с начала", () => {
    // Arrange
    const backoff = createBackoff();
    backoff.next(T0);
    backoff.next(T0 + SECOND);

    // Act
    const delay = backoff.next(T0 + FORGOTTEN_AFTER_S * SECOND);

    // Assert
    expect(delay).toBe(PAUSE.s5);
  });

  it("повтор вскоре после предыдущего продолжает расписание", () => {
    // Arrange
    const backoff = createBackoff();
    backoff.next(T0);

    // Act
    const delay = backoff.next(T0 + STILL_REMEMBERED_S * SECOND);

    // Assert
    expect(delay).toBe(PAUSE.s10);
  });

  it("ровно на границе окна забывания расписание ещё продолжается", () => {
    // Arrange
    const backoff = createBackoff();
    backoff.next(T0);

    // Act
    const delay = backoff.next(T0 + BOUNDARY_S * SECOND);

    // Assert
    expect(delay).toBe(PAUSE.s10);
  });
});
