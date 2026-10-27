import type { Preview } from "@storybook/react-vite";

type Globals = NonNullable<Preview["globalTypes"]>;

/** ВРЕМЕННО: тулбары сравнения шрифтов. Пункт `jost` — текущий шрифт кита, он же значение по умолчанию. */
export const fontGlobalTypes: Globals = {
  font: {
    description: "Шрифт",
    toolbar: {
      title: "Шрифт",
      icon: "paintbrush",
      dynamicTitle: true,
      items: [
        { value: "jost", title: "Jost (сейчас)" },
        { value: "onest", title: "Onest" },
        { value: "onest-manrope", title: "Onest + Manrope (заголовки)" },
        { value: "golos-text", title: "Golos Text" },
        { value: "inter", title: "Inter" },
        { value: "plex-sans", title: "IBM Plex Sans" },
      ],
    },
  },
  mono: {
    description: "Моно для кода",
    toolbar: {
      title: "Моно",
      icon: "code",
      dynamicTitle: true,
      items: [
        { value: "system", title: "Системный (сейчас)" },
        { value: "as-text", title: "Как текст (BRICK)" },
        { value: "jetbrains", title: "JetBrains Mono" },
        { value: "plex-mono", title: "IBM Plex Mono" },
      ],
    },
  },
};

export const fontInitialGlobals = { font: "jost", mono: "system" };

/** Ставит выбор на <html>: значения по умолчанию снимают атрибут, и CSS кита работает как в приложении. */
export function applyFonts(globals: { font?: unknown; mono?: unknown }): void {
  const root = document.documentElement;
  const font = globals.font;
  const mono = globals.mono;
  const choose = (name: string, value: unknown, fallback: string) => {
    if (typeof value === "string" && value !== fallback) {
      root.setAttribute(name, value);
    } else {
      root.removeAttribute(name);
    }
  };
  choose("data-font", font, "jost");
  choose("data-mono", mono, "system");
}
