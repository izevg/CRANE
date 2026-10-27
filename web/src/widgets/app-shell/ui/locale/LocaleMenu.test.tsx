import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it } from "vitest";
import { activateLocale } from "~/shared/lib/i18n";
import { LocaleMenu } from "./LocaleMenu";

afterEach(async () => {
  window.localStorage.removeItem("crane.locale");
  await activateLocale("ru");
});

describe("LocaleMenu", () => {
  it("кнопка называет текущий язык его же названием", () => {
    // Arrange
    // Act
    render(<LocaleMenu collapsed={false} />);

    // Assert
    expect(screen.getByRole("button", { name: "Язык: Русский" })).toBeInTheDocument();
  });

  it("пункты языков — радио с отметкой текущего", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<LocaleMenu collapsed={false} />);

    // Act
    await user.click(screen.getByRole("button", { name: "Язык: Русский" }));

    // Assert
    expect(await screen.findByRole("menuitemradio", { name: "Русский" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    expect(screen.getByRole("menuitemradio", { name: "English" })).toHaveAttribute(
      "aria-checked",
      "false",
    );
  });

  it("выбор English меняет lang документа и сохраняется в crane.locale", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<LocaleMenu collapsed={false} />);
    await user.click(screen.getByRole("button", { name: "Язык: Русский" }));

    // Act
    await user.click(await screen.findByRole("menuitemradio", { name: "English" }));

    // Assert
    await waitFor(() => expect(document.documentElement.lang).toBe("en"));
    expect(window.localStorage.getItem("crane.locale")).toBe("en");
  });

  it("после выбора English подписи интерфейса английские", async () => {
    // Arrange
    const user = userEvent.setup();
    render(<LocaleMenu collapsed={false} />);
    await user.click(screen.getByRole("button", { name: "Язык: Русский" }));

    // Act
    await user.click(await screen.findByRole("menuitemradio", { name: "English" }));

    // Assert
    expect(await screen.findByRole("button", { name: "Language: English" })).toBeInTheDocument();
  });
});
