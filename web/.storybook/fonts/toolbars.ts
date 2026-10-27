import type { Preview } from "@storybook/react-vite";

type Globals = NonNullable<Preview["globalTypes"]>;

/** ВРЕМЕННО: тулбар сравнения моно-шрифтов. Пункт `system` — текущее значение кита. */
export const fontGlobalTypes: Globals = {
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

export const fontInitialGlobals = { mono: "system" };

/** Ставит выбор на <html>: значения по умолчанию снимают атрибут, и CSS кита работает как в приложении. */
export function applyFonts(globals: { mono?: unknown }): void {
  const root = document.documentElement;
  const mono = globals.mono;
  const choose = (name: string, value: unknown, fallback: string) => {
    if (typeof value === "string" && value !== fallback) {
      root.setAttribute(name, value);
    } else {
      root.removeAttribute(name);
    }
  };
  choose("data-mono", mono, "system");
}
