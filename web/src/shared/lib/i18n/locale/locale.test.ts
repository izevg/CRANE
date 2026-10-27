import { i18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { afterAll, afterEach, describe, expect, it } from "vitest";
import { activateLocale, initLocale, setLocale } from "~/shared/lib/i18n";

const STORAGE_KEY = "crane.locale";

afterEach(() => {
  localStorage.clear();
});

afterAll(async () => {
  await activateLocale("ru");
});

describe("activateLocale", () => {
  it("включает локаль и ставит <html lang>", async () => {
    // Arrange
    const locale = "en";

    // Act
    await activateLocale(locale);

    // Assert
    expect(i18n.locale).toBe("en");
    expect(document.documentElement.lang).toBe("en");
  });

  it("не пишет выбор в хранилище", async () => {
    // Arrange
    const locale = "en";

    // Act
    await activateLocale(locale);

    // Assert
    expect(localStorage.getItem(STORAGE_KEY)).toBeNull();
  });

  it("переводит сообщение каталога на активную локаль", async () => {
    // Arrange
    await activateLocale("en");

    // Act
    const text = i18n._(msg`Скопировано`);

    // Assert
    expect(text).toBe("Copied");
  });
});

describe("setLocale", () => {
  it("запоминает выбор и включает локаль", async () => {
    // Arrange
    const locale = "en";

    // Act
    await setLocale(locale);

    // Assert
    expect(localStorage.getItem(STORAGE_KEY)).toBe("en");
    expect(i18n.locale).toBe("en");
  });
});

describe("initLocale", () => {
  it("читает сохранённую локаль", async () => {
    // Arrange
    localStorage.setItem(STORAGE_KEY, "en");

    // Act
    await initLocale();

    // Assert
    expect(i18n.locale).toBe("en");
  });

  it("без выбора и с неизвестным значением включает русский", async () => {
    // Arrange
    localStorage.setItem(STORAGE_KEY, "de");

    // Act
    await initLocale();

    // Assert
    expect(i18n.locale).toBe("ru");
  });
});
